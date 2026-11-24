//! Runs a bot built by RustBot Creator and streams its logs (D-022): in the app, or hosted
//! as a `systemd --user` service ([`service`]).
//!
//! The token reaches the bot only through its environment (D-056), never its arguments,
//! which other users can see in `ps`.

use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

pub mod service;

/// Why a bot could not be started, stopped or set up.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file, a folder or the bot's process could not be used.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file, folder or program.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The bot token is empty.
    #[error("the bot has no token")]
    NoToken,
    /// A server id is not a Discord id (digits only).
    #[error("{0:?} is not a server ID")]
    InvalidServerId(String),
    /// The hosted bot's service could not be set up or controlled (D-022).
    #[error("{0}")]
    Service(String),
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}

/// Which output of the bot a line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error, where the bot's logs go.
    Stderr,
}

/// One line the bot printed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Where it was printed.
    pub stream: Stream,
    /// The line, without its line break. Invalid UTF-8 is replaced.
    pub text: String,
}

/// A running bot. Dropping it stops the bot.
#[derive(Debug)]
pub struct Bot {
    child: Child,
}

impl Bot {
    /// Starts the bot at `binary` in its own folder with the token and, if given, the test
    /// server its commands are registered on (D-055). Each line it prints is sent to
    /// `logs` until it exits.
    ///
    /// # Errors
    /// [`Error::NoToken`], [`Error::InvalidServerId`], or the bot could not be started.
    pub fn start(
        binary: &Path,
        token: &str,
        test_server: Option<&str>,
        logs: &Sender<LogLine>,
    ) -> Result<Self, Error> {
        if token.trim().is_empty() {
            return Err(Error::NoToken);
        }
        let mut command = Command::new(binary);
        command
            .env("DISCORD_TOKEN", token.trim())
            // The Logs panel shows plain text: no colour codes (https://no-color.org).
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = binary.parent() {
            command.current_dir(dir);
        }
        match test_server {
            Some(id) => command.env("DISCORD_TEST_GUILD_ID", checked_server_id(id)?),
            None => command.env_remove("DISCORD_TEST_GUILD_ID"),
        };
        let mut child = command.spawn().map_err(|e| io_error(binary, e))?;
        if let Some(out) = child.stdout.take() {
            forward(out, Stream::Stdout, logs.clone());
        }
        if let Some(err) = child.stderr.take() {
            forward(err, Stream::Stderr, logs.clone());
        }
        Ok(Self { child })
    }

    /// How the bot exited, or `None` while it runs.
    ///
    /// # Errors
    /// Its process could not be checked.
    pub fn exit_status(&mut self) -> Result<Option<ExitStatus>, Error> {
        self.child
            .try_wait()
            .map_err(|e| io_error(Path::new("the bot"), e))
    }

    /// Asks the bot to shut down cleanly (as Ctrl+C does), and kills it if it has not
    /// exited within `grace`. A bot that has already exited just reports how.
    ///
    /// # Errors
    /// Its process could not be signalled or waited for.
    pub fn stop(&mut self, grace: Duration) -> Result<ExitStatus, Error> {
        if let Some(status) = self.exit_status()? {
            return Ok(status);
        }
        // shortcut: the `kill` program sends SIGINT, since std cannot and unsafe code is
        // denied; use a signal API instead once a dependency for it is approved, and a
        // Windows mechanism in the Windows phase. The pid cannot be reused yet: the child
        // is not reaped until we wait for it.
        let _ = Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status();
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if let Some(status) = self.exit_status()? {
                return Ok(status);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        self.child
            .wait()
            .map_err(|e| io_error(Path::new("the bot"), e))
    }
}

impl Drop for Bot {
    fn drop(&mut self) {
        let _ = self.stop(Duration::from_secs(5));
    }
}

/// Sends each line of `pipe` to `logs` from a thread of its own, until the pipe closes.
fn forward(pipe: impl Read + Send + 'static, stream: Stream, logs: Sender<LogLine>) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let mut line = Vec::new();
        while reader.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
            let text = String::from_utf8_lossy(&line)
                .trim_end_matches(['\n', '\r'])
                .to_owned();
            if logs.send(LogLine { stream, text }).is_err() {
                return;
            }
            line.clear();
        }
    });
}

fn checked_server_id(id: &str) -> Result<&str, Error> {
    let id = id.trim();
    if id.parse::<u64>().is_ok_and(|n| n > 0) && id.bytes().all(|b| b.is_ascii_digit()) {
        Ok(id)
    } else {
        Err(Error::InvalidServerId(id.to_owned()))
    }
}

/// The test server stored in the project's folder of the app's data (D-065), or `None`.
///
/// # Errors
/// The file could not be read.
pub fn test_server(bot_dir: &Path) -> Result<Option<String>, Error> {
    let path = bot_dir.join("test-server");
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text.trim().to_owned()).filter(|id| !id.is_empty())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(&path, e)),
    }
}

/// Stores the test server, or forgets it with `None`.
///
/// # Errors
/// [`Error::InvalidServerId`], or the file could not be written.
pub fn set_test_server(bot_dir: &Path, id: Option<&str>) -> Result<(), Error> {
    let path = bot_dir.join("test-server");
    let result = match id {
        Some(id) => {
            let id = checked_server_id(id)?;
            std::fs::create_dir_all(bot_dir).and_then(|()| std::fs::write(&path, format!("{id}\n")))
        }
        None => match std::fs::remove_file(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        },
    };
    result.map_err(|e| io_error(&path, e))
}
