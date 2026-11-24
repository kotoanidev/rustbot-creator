//! The hosted bot as a Task Scheduler task on Windows (D-022, D-151–D-154). The task
//! starts the launcher at the user's login and needs no administrator. The launcher
//! restarts the bot after a crash itself, since Task Scheduler does not restart a task
//! that ran and failed, and it writes the bot's output to [`super::LOG_FILE`].

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use super::{Journal, Service, State, follow_file, is_package};
use crate::{Error, LogLine, hide_console, io_error};

/// The folder of the app's tasks in Task Scheduler.
const FOLDER: &str = "RustBot Creator";

/// Task Scheduler's results that mean the task did not fail: it never ran, or the user
/// (or Run, D-141) ended it.
const NEVER_RAN: i64 = 0x0004_1303;
const ENDED: i64 = 0x0004_1306;
const ENDED_BY_CTRL_C: i64 = 0xC000_013A;

impl Service {
    /// The task's name, e.g. `rustbot-<id>`, in the folder "RustBot Creator".
    #[must_use]
    pub fn name(&self) -> String {
        format!("rustbot-{}", self.id)
    }

    fn path(&self) -> String {
        format!("\\{FOLDER}\\{}", self.name())
    }

    /// The task's definition: at the user's login, the launcher with this project's id
    /// and the bot's `package`.
    ///
    /// # Errors
    /// [`Error::Service`] if the launcher's path is not UTF-8, `package` is not a package
    /// name, or the user is unknown.
    pub fn task(&self, launcher: &Path, package: &str, bot_name: &str) -> Result<String, Error> {
        let launcher = launcher
            .to_str()
            .ok_or_else(|| Error::Service("the app's path is not UTF-8".to_owned()))?;
        if !is_package(package) {
            return Err(Error::Service(format!("{package:?} is not a package name")));
        }
        let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
            (Ok(domain), Ok(user)) => format!("{domain}\\{user}"),
            (Err(_), Ok(user)) => user,
            _ => return Err(Error::Service("the user is unknown".to_owned())),
        };
        let name: String = bot_name.chars().filter(|c| !c.is_control()).collect();
        Ok(format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Discord bot {name} (RustBot Creator). Rewritten by its "Update".</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{launcher}</Command>
      <Arguments>--host {id} {package}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
            name = xml(&name),
            user = xml(&user),
            launcher = xml(launcher),
            id = self.id,
        ))
    }

    /// Registers the task, replacing an older one, and starts it now.
    ///
    /// # Errors
    /// [`Error::Service`] or [`Error::Io`].
    pub fn install(&self, launcher: &Path, package: &str, bot_name: &str) -> Result<(), Error> {
        let task = self.task(launcher, package, bot_name)?;
        // schtasks reads the definition from a file, in UTF-16 with a byte order mark.
        let file = std::env::temp_dir().join(format!("{}.xml", self.name()));
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(task.encode_utf16().flat_map(u16::to_le_bytes));
        std::fs::write(&file, bytes).map_err(|e| io_error(&file, e))?;
        let file_arg = file.to_string_lossy().into_owned();
        let created = schtasks(&["/Create", "/TN", &self.path(), "/XML", &file_arg, "/F"]);
        let _ = std::fs::remove_file(&file);
        created?;
        self.start()
    }

    /// Starts the task.
    ///
    /// # Errors
    /// [`Error::Service`].
    pub fn start(&self) -> Result<(), Error> {
        schtasks(&["/Run", "/TN", &self.path()])
    }

    /// Ends the task, and with it the bot, until it is started again or the next login.
    ///
    /// # Errors
    /// [`Error::Service`].
    pub fn stop(&self) -> Result<(), Error> {
        schtasks(&["/End", "/TN", &self.path()])
    }

    /// Ends and deletes the task.
    ///
    /// # Errors
    /// [`Error::Service`].
    pub fn remove(&self) -> Result<(), Error> {
        let _ = self.stop();
        schtasks(&["/Delete", "/TN", &self.path(), "/F"])
    }

    /// What the task is doing. PowerShell's task states, unlike `schtasks /Query`, are not
    /// translated into the system's language. An unknown answer counts as stopped.
    #[must_use]
    pub fn state(&self) -> State {
        let script = format!(
            "$t = Get-ScheduledTask -TaskPath '\\{FOLDER}\\' -TaskName '{}' \
             -ErrorAction SilentlyContinue; \
             if ($t) {{ \"$($t.State) $(($t | Get-ScheduledTaskInfo).LastTaskResult)\" }} \
             else {{ 'Missing' }}",
            self.name()
        );
        let mut command = Command::new("powershell");
        command
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        match hide_console(&mut command).output() {
            Ok(output) => parse_state(&String::from_utf8_lossy(&output.stdout)),
            Err(_) => State::Stopped,
        }
    }

    /// Follows the launcher's log file: its last `lines` lines, then each new one, sent to
    /// `logs` until the returned [`Journal`] is dropped.
    ///
    /// # Errors
    /// None on Windows; the file may not exist yet.
    pub fn journal(
        &self,
        log_file: &Path,
        lines: usize,
        logs: &Sender<LogLine>,
    ) -> Result<Journal, Error> {
        Ok(Journal {
            stop: follow_file(log_file, lines, logs.clone()),
        })
    }
}

/// The state from PowerShell's "<state> <last result>", or "Missing".
fn parse_state(answer: &str) -> State {
    let mut words = answer.split_whitespace();
    let state = words.next().unwrap_or("");
    let result = words
        .next()
        .and_then(|r| r.parse::<i64>().ok())
        .unwrap_or(0);
    match state {
        "Missing" => State::Missing,
        "Running" => State::Running,
        "Queued" => State::Starting,
        "Ready" if ![0, NEVER_RAN, ENDED, ENDED_BY_CTRL_C].contains(&result) => State::Failed,
        _ => State::Stopped,
    }
}

/// `text` with XML's special characters escaped.
fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn schtasks(args: &[&str]) -> Result<(), Error> {
    let mut command = Command::new("schtasks");
    command.args(args).stdin(Stdio::null());
    let output = hide_console(&mut command)
        .output()
        .map_err(|e| io_error(Path::new("schtasks"), e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Service(format!(
            "schtasks {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Service, State, parse_state};

    #[test]
    fn task_states_are_read() {
        assert_eq!(parse_state("Missing\r\n"), State::Missing);
        assert_eq!(parse_state("Running 267009\r\n"), State::Running);
        assert_eq!(parse_state("Queued 0"), State::Starting);
        assert_eq!(parse_state("Ready 0"), State::Stopped);
        assert_eq!(parse_state("Ready 267014"), State::Stopped);
        assert_eq!(parse_state("Ready 78"), State::Failed);
        assert_eq!(parse_state("Disabled 0"), State::Stopped);
        assert_eq!(parse_state(""), State::Stopped);
    }

    /// Nothing from the bot's name or the app's path can add an element to the task.
    #[test]
    fn the_task_escapes_what_it_quotes() {
        let service = Service::new("0b8f1c2e-5d6a-4b7c-8e9f-0a1b2c3d4e5f").unwrap();
        let task = service
            .task(
                Path::new(r"C:\Apps & Tools\<x>\rustbot-creator.exe"),
                "my-bot",
                "Bot</Description><Exec>evil",
            )
            .unwrap();
        assert!(
            task.contains(r"<Command>C:\Apps &amp; Tools\&lt;x&gt;\rustbot-creator.exe</Command>")
        );
        assert!(task.contains("Bot&lt;/Description&gt;&lt;Exec&gt;evil"));
        assert_eq!(task.matches("<Exec>").count(), 1);
        assert!(task.contains("--host 0b8f1c2e-5d6a-4b7c-8e9f-0a1b2c3d4e5f my-bot</Arguments>"));
        assert!(service.task(Path::new("x"), "../evil", "Bot").is_err());
    }

    /// A real round trip through Task Scheduler: a task whose launcher stops at once with
    /// [`super::super::CANNOT_START`] ends as failed, and is then removed. Run by hand, and
    /// in CI on Windows.
    #[test]
    #[ignore = "registers and deletes a real Task Scheduler task"]
    fn a_task_installs_fails_and_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let launcher = dir.path().join("launcher.cmd");
        std::fs::write(&launcher, "@echo off\r\nexit /b 78\r\n").unwrap();
        let service = Service::new("00000000-0000-4000-8000-000000000001").unwrap();
        service.install(&launcher, "my-bot", "Test bot").unwrap();
        let mut state = State::Starting;
        for _ in 0..100 {
            state = service.state();
            if state == State::Failed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        service.remove().unwrap();
        assert_eq!(state, State::Failed);
        assert_eq!(service.state(), State::Missing);
    }
}
