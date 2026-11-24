//! Hosting on this PC (D-022, D-138–D-143). "Host…" puts a release snapshot of the bot in
//! its folder of the app's data and installs a `systemd --user` service, which runs the
//! launcher below at every login. The bot's `data.json` stays in that folder, shared with
//! Run, which pauses the service while it tests.

use std::fs;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use gpui_kit::{App, AppContext as _, AsyncWindowContext, Context, Entity, Window};
use rustbot_runner::LogLine;
use rustbot_runner::service::{self, Service, State};

use crate::i18n::{t, t_with};
use crate::install::ask;
use crate::run::{self, Run};
use crate::state::{AppState, Line, Source};

/// The hosted program's folder, inside the bot's folder.
const HOSTED: &str = "hosted";

/// How often the journal's new lines are moved into the Logs panel, and how many of
/// those ticks pass between looks at the service.
const TICK: Duration = Duration::from_millis(200);
const TICKS_PER_CHECK: u32 = 25;

/// The hosted bot, as last seen.
#[derive(Default)]
pub(crate) struct Hosted {
    /// `None` until the service is first checked.
    pub(crate) state: Option<State>,
    /// Run stopped it and starts it again when it ends (D-141).
    pub(crate) paused: bool,
    /// The graphs have changed since the snapshot (D-140).
    pub(crate) out_of_date: bool,
    journal: Option<service::Journal>,
}

impl Hosted {
    pub(crate) fn is_hosted(&self) -> bool {
        self.state.is_some_and(|state| state != State::Missing)
    }

    /// The status bar's text, e.g. "Hosted: running · out of date".
    pub(crate) fn label(&self) -> Option<String> {
        let key = match self.state? {
            State::Missing => return None,
            _ if self.paused => "host-state-paused",
            State::Running => "host-state-running",
            State::Starting => "host-state-starting",
            State::Stopped => "host-state-stopped",
            State::Failed => "host-state-failed",
        };
        let label = format!("⌂ {}", t(key));
        Some(if self.out_of_date {
            format!("{label} · {}", t("host-out-of-date"))
        } else {
            label
        })
    }
}

/// Whether hosting works on this system: `systemd --user` for now. Windows gets Task
/// Scheduler later in phase 11 (D-022).
pub(crate) const AVAILABLE: bool = cfg!(target_os = "linux");

/// `rustbot-creator --host <project id> <package>`, which the service runs (D-138): starts
/// the hosted program with the token from the keyring, in place of this process.
pub(crate) fn launch(id: &str, package: &str) -> ExitCode {
    let fail = |code: u8, message: &str| {
        eprintln!("{message}");
        ExitCode::from(code)
    };
    let cannot = |message: &str| fail(service::CANNOT_START, message);
    let Ok(project) = uuid::Uuid::parse_str(id) else {
        return cannot(&format!("{id:?} is not a project ID"));
    };
    if Service::new(id).is_none() || !service::is_package(package) {
        return cannot(&format!("{id} {package:?} is not a hosted bot"));
    }
    let Ok(toolchain) = rustbot_toolchain::Toolchain::locate() else {
        return cannot("The home folder is unknown.");
    };
    let dir = toolchain.bot_dir_by_id(id);
    let program = dir
        .join(HOSTED)
        .join(rustbot_toolchain::program_name(package));
    let token = match rustbot_secrets::bot_token(project) {
        Ok(Some(token)) if !token.trim().is_empty() => token,
        Ok(_) => {
            return cannot(
                "The bot has no token. Paste it in the bot settings of RustBot Creator, then \
                 start the hosted bot again.",
            );
        }
        // Right after login the keyring may not be open yet: the service tries again.
        Err(e) => return fail(1, &format!("The keyring cannot be read: {e}")),
    };
    // Its commands are global (D-055), and its data.json is the one Run uses (D-143).
    let mut command = std::process::Command::new(&program);
    command
        .current_dir(&dir)
        .env("DISCORD_TOKEN", token.trim())
        .env("NO_COLOR", "1")
        .env_remove("DISCORD_TEST_GUILD_ID");
    // The bot takes this process's place, so systemd watches and signals the bot itself.
    #[cfg(unix)]
    let error = {
        use std::os::unix::process::CommandExt as _;
        command.exec()
    };
    #[cfg(not(unix))]
    let error = match rustbot_runner::hide_console(&mut command).status() {
        Ok(status) => {
            let code = status.code().and_then(|code| u8::try_from(code).ok());
            return ExitCode::from(code.unwrap_or(1));
        }
        Err(error) => error,
    };
    cannot(&format!("{}: {error}", program.display()))
}

fn service_of(state: &AppState) -> Option<Service> {
    Service::new(&state.project.manifest.id.to_string())
}

/// The snapshot's folder, or `None` without a home folder.
fn hosted_dir(state: &AppState) -> Option<PathBuf> {
    let toolchain = state.toolchain.as_ref()?;
    Some(toolchain.bot_dir(&state.project).join(HOSTED))
}

/// A fingerprint of the generated code, to tell an out-of-date snapshot (D-140).
// The std hasher may change with Rust; a new app then calls the snapshot out of date
// once, which its new code generator may well make true.
fn fingerprint(output: &rustbot_codegen::Output) -> String {
    let mut hasher = DefaultHasher::new();
    output.files.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Watches the hosted bot while the project is open: its state every 5 s, and its
/// journal in the Logs panel (D-143).
pub(crate) fn watch(cx: &mut Context<'_, AppState>) {
    if !AVAILABLE {
        return;
    }
    let (logs, lines) = mpsc::channel::<LogLine>();
    cx.spawn(async move |this, cx| {
        let mut ticks = 0;
        loop {
            if ticks % TICKS_PER_CHECK == 0 {
                let Ok(Some((service, dir))) = this.read_with(cx, |state, _| {
                    Some((service_of(state)?, hosted_dir(state)?))
                }) else {
                    return;
                };
                let seen = cx
                    .background_spawn(async move {
                        let version = fs::read_to_string(dir.join("version")).ok();
                        (service.state(), version)
                    })
                    .await;
                if this
                    .update(cx, |state, cx| saw(state, seen, &logs, cx))
                    .is_err()
                {
                    return;
                }
            }
            ticks += 1;
            cx.background_executor().timer(TICK).await;
            let batch: Vec<LogLine> = lines.try_iter().collect();
            if !batch.is_empty() {
                let pushed = this.update(cx, |state, cx| {
                    for line in batch {
                        let mut line = Line::output(Source::Bot, line.text, false);
                        line.text = format!("[{}] {}", t("log-hosted"), line.text);
                        state.push(line);
                    }
                    cx.notify();
                });
                if pushed.is_err() {
                    return;
                }
            }
        }
    })
    .detach();
}

fn saw(
    state: &mut AppState,
    (seen, version): (State, Option<String>),
    logs: &Sender<LogLine>,
    cx: &mut Context<'_, AppState>,
) {
    if seen == State::Missing {
        state.hosted.journal = None;
        state.hosted.out_of_date = false;
    } else {
        if state.hosted.journal.is_none() {
            state.hosted.journal = service_of(state).and_then(|s| s.journal(30, logs).ok());
        }
        // Graphs with problems generate nothing; the last answer stands then.
        if let Ok(output) = rustbot_codegen::generate(&state.project, &state.registry) {
            state.hosted.out_of_date = version.is_none_or(|v| v.trim() != fingerprint(&output));
        }
    }
    state.hosted.state = Some(seen);
    cx.notify();
}

/// "Host…" and "Update" (D-140): a release snapshot, then the service. The first time asks
/// first.
pub(crate) fn host(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    if !matches!(state.read(cx).run, Run::Stopped) {
        return;
    }
    let first = !state.read(cx).hosted.is_hosted();
    window
        .spawn(cx, async move |cx| {
            let detail = t("host-confirm-detail");
            if first && !ask(cx, &t("host-confirm-title"), &detail, &t("host-confirm")).await {
                return;
            }
            if !run::begin(&state, cx, &t("host-busy")) {
                return;
            }
            let result = snapshot(&state, cx).await;
            if result.is_ok() {
                state.update(cx, |state, _| {
                    state.hosted.state = Some(State::Starting);
                    state.hosted.out_of_date = false;
                });
            }
            run::finish(&state, cx, result.map(|()| t("host-done")));
        })
        .detach();
}

async fn snapshot(state: &Entity<AppState>, cx: &mut AsyncWindowContext) -> Result<(), String> {
    let (project, output, program) = crate::export::release(state, cx).await?;
    let failed = |e: String| t_with("error-host", &[("error", &e)]);
    let (service, dir) = state
        .read_with(cx, |state, _| {
            Some((service_of(state)?, hosted_dir(state)?))
        })
        .ok_or_else(|| t("error-no-home"))?;
    let launcher = std::env::current_exe().map_err(|e| failed(e.to_string()))?;
    let version = fingerprint(&output);
    let (package, name) = (project.manifest.bot.package, project.manifest.bot.name);
    cx.background_spawn(async move {
        let io = |e: std::io::Error| e.to_string();
        fs::create_dir_all(&dir).map_err(io)?;
        // Copy, then rename: the running snapshot keeps its file until the restart.
        let tmp = dir.join(format!(".{package}.tmp"));
        fs::copy(&program, &tmp).map_err(io)?;
        fs::rename(&tmp, dir.join(rustbot_toolchain::program_name(&package))).map_err(io)?;
        fs::write(dir.join("version"), version).map_err(io)?;
        service
            .install(&launcher, &package, &name)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(failed)
}

/// Stop or Start in the bot settings.
pub(crate) fn switch(state: &Entity<AppState>, on: bool, cx: &mut App) {
    let Some(service) = service_of(state.read(cx)) else {
        return;
    };
    let state = state.clone();
    cx.spawn(async move |cx| {
        let (result, seen) = cx
            .background_spawn(async move {
                let result = if on { service.start() } else { service.stop() };
                (result, service.state())
            })
            .await;
        state.update(cx, |state, cx| {
            if let Err(e) = result {
                state.log_error(t_with("error-host", &[("error", &e.to_string())]));
            }
            state.hosted.state = Some(seen);
            cx.notify();
        });
    })
    .detach();
}

/// "Remove…": the service and the snapshot go, and the memory stays.
pub(crate) fn remove(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    let Some((service, dir)) = (|| {
        let state = state.read(cx);
        Some((service_of(state)?, hosted_dir(state)?))
    })() else {
        return;
    };
    window
        .spawn(cx, async move |cx| {
            let (title, detail) = (t("host-remove-title"), t("host-remove-detail"));
            if !ask(cx, &title, &detail, &t("host-remove")).await {
                return;
            }
            let result = cx
                .background_spawn(async move {
                    service.remove().map_err(|e| e.to_string())?;
                    match fs::remove_dir_all(&dir) {
                        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                        _ => Ok(()),
                    }
                })
                .await;
            state.update(cx, |state, cx| {
                match result {
                    Ok(()) => state.log(t("host-removed")),
                    Err(e) => state.log_error(t_with("error-host", &[("error", &e)])),
                }
                state.hosted = Hosted {
                    state: Some(State::Missing),
                    ..Hosted::default()
                };
                cx.notify();
            });
        })
        .detach();
}

/// Stops the hosted bot while Run tests this one (D-141).
pub(crate) async fn pause(state: &Entity<AppState>, cx: &mut AsyncWindowContext) {
    let Some(service) = state.read_with(cx, |state, _| service_of(state)) else {
        return;
    };
    // Asked now, not from the last look: the service may have just started at login.
    let stopped = cx
        .background_spawn(async move {
            match service.state() {
                State::Running | State::Starting => service.stop().map(|()| true),
                _ => Ok(false),
            }
        })
        .await;
    state.update(cx, |state, cx| {
        match stopped {
            Ok(true) => {
                state.hosted.paused = true;
                state.hosted.state = Some(State::Stopped);
                state.log(t("host-paused-log"));
            }
            Ok(false) => {}
            Err(e) => state.log_error(t_with("error-host", &[("error", &e.to_string())])),
        }
        cx.notify();
    });
}

/// Starts the hosted bot again after Run, if Run paused it.
pub(crate) async fn resume(state: &Entity<AppState>, cx: &mut AsyncWindowContext) {
    let Some(service) = state.read_with(cx, |state, _| {
        state.hosted.paused.then(|| service_of(state)).flatten()
    }) else {
        return;
    };
    let started = cx.background_spawn(async move { service.start() }).await;
    state.update(cx, |state, cx| {
        state.hosted.paused = false;
        match started {
            Ok(()) => {
                state.hosted.state = Some(State::Starting);
                state.log(t("host-resumed-log"));
            }
            Err(e) => state.log_error(t_with("error-host", &[("error", &e.to_string())])),
        }
        cx.notify();
    });
}

/// When the app quits during Run: the test bot stops first, then the hosted one starts
/// again, so the bot is not left offline until the next login.
pub(crate) fn resume_on_quit(state: &mut AppState) {
    if !state.hosted.paused {
        return;
    }
    // Dropping the running bot stops it.
    state.run = Run::Stopped;
    if let Some(service) = service_of(state) {
        let _ = service.start();
    }
    state.hosted.paused = false;
}

#[cfg(test)]
mod tests {
    use rustbot_runner::service::State;

    use super::Hosted;

    #[test]
    fn the_label_tells_the_state_and_an_old_snapshot() {
        assert_eq!(Hosted::default().label(), None);
        let missing = Hosted {
            state: Some(State::Missing),
            ..Hosted::default()
        };
        assert_eq!(missing.label(), None);
        let paused = Hosted {
            state: Some(State::Stopped),
            paused: true,
            out_of_date: true,
            ..Hosted::default()
        };
        let label = paused.label().unwrap();
        assert!(label.starts_with("⌂ "));
        assert!(label.contains(&crate::i18n::t("host-state-paused")));
        assert!(label.contains(&crate::i18n::t("host-out-of-date")));
    }

    /// The launcher refuses what is not a hosted bot, without a retry, and never runs a
    /// path out of a package name.
    #[test]
    fn the_launcher_refuses_what_is_not_a_hosted_bot() {
        let cannot = std::process::ExitCode::from(rustbot_runner::service::CANNOT_START);
        assert_eq!(super::launch("not-a-uuid", "bot"), cannot);
        let id = "0b8f1c2e-5d6a-4b7c-8e9f-0a1b2c3d4e5f";
        assert_eq!(super::launch(id, "../../bin/sh"), cannot);
    }
}
