//! The hosted bot as a `systemd --user` service on Linux (D-138–D-143). It starts at
//! login, restarts after a crash, and logs to the journal.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use super::{CANNOT_START, Journal, Service, State, is_package};
use crate::{Error, LogLine, Stream, forward, io_error};

impl Service {
    /// The unit's name, e.g. `rustbot-<id>.service`.
    #[must_use]
    pub fn name(&self) -> String {
        format!("rustbot-{}.service", self.id)
    }

    /// Where systemd looks for the user's own units.
    fn unit_path(&self) -> Result<PathBuf, Error> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or_else(|| Error::Service("the home folder is unknown".to_owned()))?;
        Ok(config.join("systemd").join("user").join(self.name()))
    }

    /// The unit file: the launcher with this project's id and the bot's `package`.
    ///
    /// # Errors
    /// [`Error::Service`] if the launcher's path is not UTF-8 or `package` is not a
    /// package name.
    pub fn unit(&self, launcher: &Path, package: &str, bot_name: &str) -> Result<String, Error> {
        let launcher = launcher
            .to_str()
            .ok_or_else(|| Error::Service("the app's path is not UTF-8".to_owned()))?;
        if !is_package(package) {
            return Err(Error::Service(format!("{package:?} is not a package name")));
        }
        // `%` starts a specifier and `$` a variable in unit files; control characters
        // could start a new line, and so a new setting.
        let name: String = bot_name.chars().filter(|c| !c.is_control()).collect();
        let quoted = launcher
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$");
        Ok(format!(
            "# Made by RustBot Creator, and rewritten by its \"Update\". Do not edit.\n\
             [Unit]\n\
             Description=Discord bot {name} (RustBot Creator)\n\
             \n\
             [Service]\n\
             ExecStart=\"{quoted}\" --host {id} {package}\n\
             Restart=on-failure\n\
             RestartSec=10\n\
             RestartPreventExitStatus={CANNOT_START}\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n",
            name = name.replace('%', "%%"),
            id = self.id,
        ))
    }

    /// Writes the unit, then enables the service for every login and (re)starts it now.
    ///
    /// # Errors
    /// [`Error::Service`] or [`Error::Io`].
    pub fn install(&self, launcher: &Path, package: &str, bot_name: &str) -> Result<(), Error> {
        let unit = self.unit(launcher, package, bot_name)?;
        let path = self.unit_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io_error(dir, e))?;
        }
        let tmp = path.with_extension("service.tmp");
        std::fs::write(&tmp, unit).map_err(|e| io_error(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| io_error(&path, e))?;
        systemctl(&["daemon-reload"])?;
        systemctl(&["enable", &self.name()])?;
        systemctl(&["restart", &self.name()])
    }

    /// Starts the service.
    ///
    /// # Errors
    /// [`Error::Service`].
    pub fn start(&self) -> Result<(), Error> {
        systemctl(&["start", &self.name()])
    }

    /// Stops the service until it is started again or the next login.
    ///
    /// # Errors
    /// [`Error::Service`].
    pub fn stop(&self) -> Result<(), Error> {
        systemctl(&["stop", &self.name()])
    }

    /// Stops and disables the service, and deletes its unit.
    ///
    /// # Errors
    /// [`Error::Service`] or [`Error::Io`].
    pub fn remove(&self) -> Result<(), Error> {
        // A unit systemd has not loaded cannot be disabled; deleting it is enough then.
        let _ = systemctl(&["disable", "--now", &self.name()]);
        let path = self.unit_path()?;
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(io_error(&path, e)),
            _ => {}
        }
        systemctl(&["daemon-reload"])?;
        let _ = systemctl(&["reset-failed", &self.name()]);
        Ok(())
    }

    /// What the service is doing. An unknown answer counts as stopped.
    #[must_use]
    pub fn state(&self) -> State {
        if !self.unit_path().is_ok_and(|path| path.exists()) {
            return State::Missing;
        }
        let answer = Command::new("systemctl")
            .args(["--user", "is-active", &self.name()])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        let answer = answer.map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
        match answer.as_deref() {
            Ok("active") => State::Running,
            Ok("activating" | "reloading") => State::Starting,
            Ok("failed") => State::Failed,
            _ => State::Stopped,
        }
    }

    /// Follows the service's journal: its last `lines` lines, then each new one, sent to
    /// `logs` until the returned [`Journal`] is dropped. The log file is for Windows,
    /// which has no journal.
    ///
    /// # Errors
    /// [`Error::Io`] if `journalctl` cannot be started.
    pub fn journal(
        &self,
        _log_file: &Path,
        lines: usize,
        logs: &Sender<LogLine>,
    ) -> Result<Journal, Error> {
        let mut child = Command::new("journalctl")
            .args([
                "--user",
                "--unit",
                &self.name(),
                "--output",
                "cat",
                "--follow",
            ])
            .args(["--no-pager", "--lines", &lines.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| io_error(Path::new("journalctl"), e))?;
        if let Some(out) = child.stdout.take() {
            // The bot logs to stderr, and the journal keeps the bot's lines only.
            forward(out, Stream::Stderr, logs.clone());
        }
        Ok(Journal { child })
    }
}

fn systemctl(args: &[&str]) -> Result<(), Error> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| io_error(Path::new("systemctl"), e))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Service(format!(
            "systemctl --user {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Service;

    const ID: &str = "0b8f1c2e-5d6a-4b7c-8e9f-0a1b2c3d4e5f";

    /// Nothing from the bot's name or the app's path can add a setting or a specifier.
    #[test]
    fn the_unit_escapes_what_it_quotes() {
        let service = Service::new(ID).unwrap();
        let unit = service
            .unit(
                Path::new("/opt/my apps/\"x\"/50%$HOME/rustbot-creator"),
                "my-bot",
                "Bot\nExecStartPre=/bin/evil %h",
            )
            .unwrap();
        assert!(
            unit.contains("ExecStart=\"/opt/my apps/\\\"x\\\"/50%%$$HOME/rustbot-creator\" --host")
        );
        assert!(unit.contains(&format!("--host {ID} my-bot\n")));
        assert!(unit.contains("Description=Discord bot BotExecStartPre=/bin/evil %%h"));
        assert_eq!(unit.matches("ExecStart").count(), 2, "{unit}");
        assert!(unit.lines().all(|line| !line.starts_with("ExecStartPre")));
        assert!(unit.contains("RestartPreventExitStatus=78"));
        assert!(service.unit(Path::new("/x"), "../evil", "Bot").is_err());
    }

    /// A real round trip through `systemctl --user`: a service whose launcher stops at
    /// once with [`super::CANNOT_START`] ends as failed and is not restarted. Then it is
    /// removed. Run by hand (see CLAUDE.md).
    #[cfg(unix)]
    #[test]
    #[ignore = "installs and removes a real systemd --user unit"]
    fn a_service_installs_fails_without_restarts_and_is_removed() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let launcher = dir.path().join("launcher");
        std::fs::write(
            &launcher,
            "#!/bin/sh\necho \"no token for $2\" >&2\nexit 78\n",
        )
        .unwrap();
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755)).unwrap();
        let service = Service::new("00000000-0000-4000-8000-000000000001").unwrap();
        service.install(&launcher, "my-bot", "Test bot").unwrap();
        let mut state = super::State::Starting;
        for _ in 0..50 {
            state = service.state();
            if state == super::State::Failed {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let (logs, lines) = std::sync::mpsc::channel();
        let journal = service.journal(Path::new("unused"), 20, &logs).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        drop(journal);
        let lines: Vec<String> = lines.try_iter().map(|line| line.text).collect();
        service.remove().unwrap();
        assert_eq!(state, super::State::Failed);
        assert_eq!(service.state(), super::State::Missing);
        assert!(
            lines.iter().any(|line| line.contains("no token for")),
            "{lines:?}"
        );
    }
}
