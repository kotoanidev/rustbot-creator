//! Run and Stop (D-071): save, check, generate, build and start the bot, asking first
//! before anything is trusted or installed (D-039, D-064). Every step's output goes to
//! the Logs panel.

use std::sync::mpsc;
use std::time::Duration;

use gpui_kit::{App, AppContext as _, AsyncWindowContext, Entity, Window};
use rustbot_runner::Bot;

use crate::i18n::{t, t_with};
use crate::install::{Progress, ask, install, logged};
use crate::state::{AppState, Line, Source};
use crate::trust;

/// How often the bot's output is moved into the Logs panel.
const LOG_TICK: Duration = Duration::from_millis(50);

/// What the bot is doing, for the status bar.
pub(crate) enum Run {
    Stopped,
    /// A step is in progress; the text says which.
    Busy(String),
    Running(Bot),
}

/// Runs the bot, unless it is already running or starting.
pub(crate) fn start(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    if !matches!(state.read(cx).run, Run::Stopped) {
        return;
    }
    window
        .spawn(cx, async move |cx| {
            if let Err(message) = run(&state, cx).await {
                state.update(cx, |state, cx| {
                    state.log_error(message);
                    state.run = Run::Stopped;
                    cx.notify();
                });
            }
        })
        .detach();
}

/// Asks the running bot to stop, and kills it if it does not within 5 s.
pub(crate) fn stop(state: &Entity<AppState>, cx: &mut App) {
    let bot = state.update(cx, |state, cx| {
        let busy = Run::Busy(t("run-stopping"));
        cx.notify();
        match std::mem::replace(&mut state.run, busy) {
            Run::Running(bot) => Some(bot),
            other => {
                state.run = other;
                None
            }
        }
    });
    let Some(mut bot) = bot else { return };
    let stopped = cx.background_spawn(async move { bot.stop(Duration::from_secs(5)) });
    let state = state.clone();
    cx.spawn(async move |cx| {
        let status = match stopped.await {
            Ok(status) => status.to_string(),
            Err(e) => e.to_string(),
        };
        state.update(cx, |state, cx| {
            state.log(t_with("run-exited", &[("status", &status)]));
            state.run = Run::Stopped;
            cx.notify();
        });
    })
    .detach();
}

/// Marks the bot busy with `text` for a task besides Run, e.g. an export, unless it already
/// runs or is busy: buttons may have been pressed while a dialog was open. Returns whether
/// the task may go on.
pub(crate) fn begin(state: &Entity<AppState>, cx: &mut AsyncWindowContext, text: &str) -> bool {
    state.update(cx, |state, cx| {
        let stopped = matches!(state.run, Run::Stopped);
        if stopped {
            state.run = Run::Busy(text.to_owned());
            cx.notify();
        }
        stopped
    })
}

/// Ends a task [`begin`] started, logging its message or error.
pub(crate) fn finish(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
    result: Result<String, String>,
) {
    state.update(cx, |state, cx| {
        match result {
            Ok(message) => state.log(message),
            Err(message) => state.log_error(message),
        }
        state.run = Run::Stopped;
        cx.notify();
    });
}

pub(crate) fn busy(state: &Entity<AppState>, cx: &mut AsyncWindowContext, text: &str) {
    state.update(cx, |state, cx| {
        state.log(text.to_owned());
        state.run = Run::Busy(text.to_owned());
        cx.notify();
    });
}

/// Install and build progress, into the status bar and the Logs panel.
pub(crate) fn report(state: &Entity<AppState>) -> impl Fn(&mut AsyncWindowContext, Progress) + '_ {
    move |cx, progress| match progress {
        Progress::Step(text) => busy(state, cx, &text),
        Progress::Lines(lines) => state.update(cx, |state, cx| {
            for line in lines {
                state.push(Line::output(Source::Build, line, true));
            }
            cx.notify();
        }),
    }
}

async fn run(state: &Entity<AppState>, cx: &mut AsyncWindowContext) -> Result<(), String> {
    let (project, toolchain) = check(state, cx).await?;
    let report = report(state);
    install(cx, &toolchain, &report).await?;
    generate(state, cx)?;

    busy(state, cx, &t("run-building"));
    let (builder, built) = (toolchain.clone(), project.clone());
    let binary = logged(cx, &report, move |log| builder.build(&built, log))
        .await
        .map_err(|e| t_with("error-build", &[("error", &e.to_string())]))?;

    busy(state, cx, &t("run-starting"));
    let id = project.manifest.id;
    let token = cx
        .background_spawn(async move { rustbot_secrets::bot_token(id) })
        .await
        .map_err(|e| t_with("error-keyring", &[("error", &e.to_string())]))?
        .ok_or_else(|| t("run-no-token"))?;
    let server = rustbot_runner::test_server(&toolchain.bot_dir(&project))
        .ok()
        .flatten();
    let (lines_tx, lines) = mpsc::channel();
    // One token answers once: the hosted copy waits while this one runs (D-141).
    crate::host::pause(state, cx).await;
    crate::publish::pause(state, cx).await;
    let bot = match Bot::start(&binary, &token, server.as_deref(), &lines_tx) {
        Ok(bot) => bot,
        Err(e) => {
            crate::host::resume(state, cx).await;
            crate::publish::resume(state, cx).await;
            return Err(t_with("error-start", &[("error", &e.to_string())]));
        }
    };
    drop(token);
    state.update(cx, |state, cx| {
        state.log(t("run-started"));
        state.run = Run::Running(bot);
        cx.notify();
    });
    follow(state, cx, &lines).await;
    crate::host::resume(state, cx).await;
    crate::publish::resume(state, cx).await;
    Ok(())
}

/// Writes `generated/` from the graphs, and returns what was written.
pub(crate) fn generate(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
) -> Result<rustbot_codegen::Output, String> {
    busy(state, cx, &t("run-generating"));
    state
        .update(cx, |state, _| {
            let output = rustbot_codegen::generate(&state.project, &state.registry)
                .map_err(|e| e.to_string())?;
            rustbot_codegen::write(&state.project, &output).map_err(|e| e.to_string())?;
            Ok(output)
        })
        .map_err(|e: String| t_with("error-codegen", &[("error", &e)]))
}

/// Saves and checks for problems.
pub(crate) fn checked(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
) -> Result<rustbot_core::Project, String> {
    busy(state, cx, &t("run-checking"));
    state.update(cx, |_, cx| cx.emit(crate::state::SaveCode));
    let (project, problems) = state.update(cx, |state, _| {
        state.save_now();
        let problems = state
            .project
            .graphs
            .iter()
            .map(|file| state.registry.validate(&file.graph).len())
            .sum::<usize>();
        (state.project.clone(), problems)
    });
    if problems > 0 {
        return Err(t("run-has-problems"));
    }
    Ok(project)
}

/// [`checked`], then asks before building a project from elsewhere (D-064).
pub(crate) async fn check(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
) -> Result<(rustbot_core::Project, rustbot_toolchain::Toolchain), String> {
    let project = checked(state, cx)?;
    let toolchain = state
        .read_with(cx, |state, _| state.toolchain.clone())
        .ok_or_else(|| t("error-no-home"))?;
    if !trust::is_trusted(toolchain.data_dir(), &project.root) {
        let own = project
            .files_under(std::path::Path::new("src"), &[])
            .map_or(0, |files| files.len());
        let detail = t_with("trust-detail", &[("files", &own.to_string())]);
        if !ask(cx, &t("trust-title"), &detail, &t("trust-build")).await {
            return Err(t("run-cancelled"));
        }
        trust::trust(toolchain.data_dir(), &project.root)
            .map_err(|e| t_with("error-trust", &[("error", &e.to_string())]))?;
    }
    Ok((project, toolchain))
}

/// Moves the bot's output into the logs until it exits or is stopped.
async fn follow(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
    lines: &mpsc::Receiver<rustbot_runner::LogLine>,
) {
    loop {
        cx.background_executor().timer(LOG_TICK).await;
        let batch: Vec<_> = lines.try_iter().collect();
        let done = state.update(cx, |state, cx| {
            for line in batch {
                let stderr = line.stream == rustbot_runner::Stream::Stderr;
                state.push(Line::output(Source::Bot, line.text, stderr));
            }
            cx.notify();
            let Run::Running(bot) = &mut state.run else {
                return true;
            };
            match bot.exit_status() {
                Ok(None) => false,
                Ok(Some(status)) => {
                    state.log(t_with("run-exited", &[("status", &status.to_string())]));
                    state.run = Run::Stopped;
                    true
                }
                Err(e) => {
                    state.log_error(e.to_string());
                    true
                }
            }
        });
        if done {
            return;
        }
    }
}
