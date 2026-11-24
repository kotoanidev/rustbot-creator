//! Installing what bots need to build, each after asking: the system's gcc (D-039), then
//! the private Rust (D-017). Run, the first-run wizard (D-128) and Settings (D-130) use it,
//! each showing the progress its own way.

use std::sync::mpsc;
use std::time::Duration;

use gpui_kit::{AppContext as _, AsyncWindowContext, PromptLevel};

use crate::i18n::{t, t_with};

/// How often a background step's output is passed on.
const TICK: Duration = Duration::from_millis(50);

/// What an install or build step reports.
pub(crate) enum Progress {
    /// A step began; the text says which.
    Step(String),
    /// Lines of its output.
    Lines(Vec<String>),
}

/// Where the progress goes.
pub(crate) type Report<'a> = &'a dyn Fn(&mut AsyncWindowContext, Progress);

/// Asks a yes/no question in a dialog; `true` for `yes`.
pub(crate) async fn ask(cx: &mut AsyncWindowContext, title: &str, detail: &str, yes: &str) -> bool {
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

/// Runs `work` on a background thread, reporting the lines it logs as they come, and
/// returns its result.
pub(crate) async fn logged<R: Send + 'static>(
    cx: &mut AsyncWindowContext,
    report: Report<'_>,
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
        cx.background_executor().timer(TICK).await;
        let result = done.try_recv().ok();
        let batch: Vec<String> = lines.try_iter().collect();
        if !batch.is_empty() {
            report(cx, Progress::Lines(batch));
        }
        if let Some(result) = result {
            return result;
        }
    }
}

/// Installs gcc and the private Rust when they are missing, each after asking. The error
/// is the message to show, e.g. that the user cancelled.
pub(crate) async fn install(
    cx: &mut AsyncWindowContext,
    toolchain: &rustbot_toolchain::Toolchain,
    report: Report<'_>,
) -> Result<(), String> {
    if rustbot_toolchain::c_compiler().is_none() {
        if !ask(cx, &t("gcc-title"), &t("gcc-detail"), &t("gcc-install")).await {
            return Err(t("run-cancelled"));
        }
        let mut command = rustbot_toolchain::install_c_compiler_command()
            .ok_or_else(|| t("gcc-unknown-system"))?;
        report(cx, Progress::Step(t("gcc-installing")));
        let installed = logged(cx, report, move |log| match command.output() {
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
        report(cx, Progress::Step(t("rust-installing")));
        let installer = toolchain.clone();
        logged(cx, report, move |log| installer.install(log))
            .await
            .map_err(|e| t_with("error-install", &[("error", &e.to_string())]))?;
    }
    Ok(())
}
