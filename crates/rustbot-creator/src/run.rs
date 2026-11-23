//! Run and Stop (D-071): save, check, generate, build and start the bot, asking first
//! before anything is trusted or installed (D-039, D-064). Every step's output goes to
//! the Logs panel.

use std::sync::mpsc;
use std::time::Duration;

use gpui_kit::{App, AppContext as _, AsyncWindowContext, Entity, PromptLevel, Window};
use rustbot_runner::Bot;

use crate::i18n::{t, t_with};
use crate::state::{AppState, Line, Source};
use crate::trust;

/// How often the bot's and the build's output is moved into the Logs panel.
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

fn busy(state: &Entity<AppState>, cx: &mut AsyncWindowContext, text: &str) {
    state.update(cx, |state, cx| {
        state.log(text.to_owned());
        state.run = Run::Busy(text.to_owned());
        cx.notify();
    });
}

/// Asks a yes/no question in a dialog; `true` for `yes`.
async fn ask(cx: &mut AsyncWindowContext, title: &str, detail: &str, yes: &str) -> bool {
    let cancel = t("cancel");
    let Ok(answer) = cx.update(|window, cx| {
        window.prompt(
            PromptLevel::Warning,
            title,
            Some(detail),
            &[yes, cancel.as_str()],
            cx,
        )
    }) else {
        return false;
    };
    answer.await == Ok(0)
}

/// Runs `work` on a background thread, moving the lines it logs into the Logs panel as
/// they come, and returns its result.
async fn logged<R: Send + 'static>(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
    work: impl FnOnce(&mut dyn FnMut(&str)) -> R + Send + 'static,
) -> R {
    let (lines_tx, lines) = mpsc::channel::<String>();
    let (done_tx, done) = mpsc::channel::<R>();
    cx.background_spawn(async move {
        let result = work(&mut |line| {
            let _ = lines_tx.send(line.to_owned());
        });
        let _ = done_tx.send(result);
    })
    .detach();
    loop {
        cx.background_executor().timer(LOG_TICK).await;
        let result = done.try_recv().ok();
        let batch: Vec<String> = lines.try_iter().collect();
        state.update(cx, |state, cx| {
            for line in batch {
                state.push(Line::output(Source::Build, line, true));
            }
            cx.notify();
        });
        if let Some(result) = result {
            return result;
        }
    }
}

async fn run(state: &Entity<AppState>, cx: &mut AsyncWindowContext) -> Result<(), String> {
    let (project, toolchain) = check(state, cx).await?;
    install(state, cx, &toolchain).await?;

    busy(state, cx, &t("run-generating"));
    state
        .update(cx, |state, _| {
            let output = rustbot_codegen::generate(&state.project, &state.registry)
                .map_err(|e| e.to_string())?;
            rustbot_codegen::write(&state.project, &output).map_err(|e| e.to_string())
        })
        .map_err(|e: String| t_with("error-codegen", &[("error", &e)]))?;

    busy(state, cx, &t("run-building"));
    let (builder, built) = (toolchain.clone(), project.clone());
    let binary = logged(state, cx, move |log| builder.build(&built, log))
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
    let bot = Bot::start(&binary, &token, server.as_deref(), &lines_tx)
        .map_err(|e| t_with("error-start", &[("error", &e.to_string())]))?;
    drop(token);
    state.update(cx, |state, cx| {
        state.log(t("run-started"));
        state.run = Run::Running(bot);
        cx.notify();
    });
    follow(state, cx, &lines).await;
    Ok(())
}

/// Saves, checks for problems, and asks before building a project from elsewhere
/// (D-064).
async fn check(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
) -> Result<(rustbot_core::Project, rustbot_toolchain::Toolchain), String> {
    busy(state, cx, &t("run-checking"));
    let (project, problems, toolchain) = state.update(cx, |state, _| {
        state.save_now();
        let problems = state
            .project
            .graphs
            .iter()
            .map(|file| state.registry.validate(&file.graph).len())
            .sum::<usize>();
        (state.project.clone(), problems, state.toolchain.clone())
    });
    if problems > 0 {
        return Err(t("run-has-problems"));
    }
    let toolchain = toolchain.ok_or_else(|| t("error-no-home"))?;
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

/// Installs gcc (D-039) and the private Rust (D-017) when they are missing, each after
/// asking.
async fn install(
    state: &Entity<AppState>,
    cx: &mut AsyncWindowContext,
    toolchain: &rustbot_toolchain::Toolchain,
) -> Result<(), String> {
    if rustbot_toolchain::c_compiler().is_none() {
        if !ask(cx, &t("gcc-title"), &t("gcc-detail"), &t("gcc-install")).await {
            return Err(t("run-cancelled"));
        }
        let mut command = rustbot_toolchain::install_c_compiler_command()
            .ok_or_else(|| t("gcc-unknown-system"))?;
        busy(state, cx, &t("gcc-installing"));
        let installed = logged(state, cx, move |log| match command.output() {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout).into_owned()
                    + &String::from_utf8_lossy(&output.stderr);
                text.lines().for_each(&mut *log);
                output.status.success()
            }
            Err(e) => {
                log(&e.to_string());
                false
            }
        })
        .await;
        if !installed || rustbot_toolchain::c_compiler().is_none() {
            return Err(t("gcc-failed"));
        }
    }
    if !toolchain.is_installed() {
        let detail = t_with("rust-detail", &[("version", rustbot_core::RUST_VERSION)]);
        if !ask(cx, &t("rust-title"), &detail, &t("rust-install")).await {
            return Err(t("run-cancelled"));
        }
        busy(state, cx, &t("rust-installing"));
        let installer = toolchain.clone();
        logged(state, cx, move |log| installer.install(log))
            .await
            .map_err(|e| t_with("error-install", &[("error", &e.to_string())]))?;
    }
    Ok(())
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
