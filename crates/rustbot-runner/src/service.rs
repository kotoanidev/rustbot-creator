//! A hosted bot as a background service that starts at login (D-022, D-138–D-143): a
//! `systemd --user` service on Linux, and a Task Scheduler task on Windows (D-151–D-154).
//! The service runs the app's launcher, which takes the token from the keyring, so the
//! service's definition holds no secret.

use std::fs;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::{LogLine, Stream};

#[cfg(not(windows))]
mod systemd;
#[cfg(windows)]
mod task;

/// The launcher's exit code when retrying cannot help, e.g. without a token
/// (`EX_CONFIG`). The service does not restart after it.
pub const CANNOT_START: u8 = 78;

/// The hosted bot's log file in its folder, on Windows, where there is no journal. The
/// launcher writes it, and the app follows it.
pub const LOG_FILE: &str = "bot.log";

/// What the service is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The bot is not hosted: there is no service.
    Missing,
    /// The bot runs.
    Running,
    /// It is starting, or waiting to restart after a crash.
    Starting,
    /// Stopped by the user, or by Run while it tests the bot (D-141).
    Stopped,
    /// It exited with an error and is not restarted.
    Failed,
}

/// The service of one project's bot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    id: String,
}

impl Service {
    /// The service of the project with the id `id`. `None` unless it looks like a UUID,
    /// so it is safe in a service name and a path.
    #[must_use]
    pub fn new(id: &str) -> Option<Self> {
        let valid = !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || b == b'-');
        valid.then(|| Self { id: id.to_owned() })
    }
}

/// A followed log. Dropping it stops following.
#[derive(Debug)]
pub struct Journal {
    #[cfg(not(windows))]
    child: std::process::Child,
    #[cfg(windows)]
    stop: Arc<AtomicBool>,
}

impl Drop for Journal {
    fn drop(&mut self) {
        #[cfg(not(windows))]
        {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        #[cfg(windows)]
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Whether `name` is a Cargo package name, and so safe as an argument and a file name.
#[must_use]
pub fn is_package(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Follows the file at `path` from a thread of its own, as `tail -f` does: its last
/// `lines` lines, then each new one, sent to `logs` until `stop` is set. A file that
/// shrinks was rotated, and is read again from its start.
#[cfg_attr(not(windows), allow(dead_code))] // Linux follows the journal instead
fn follow_file(path: &Path, lines: usize, logs: Sender<LogLine>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let path = path.to_owned();
    std::thread::spawn(move || {
        let send = |text: &str| {
            logs.send(LogLine {
                stream: Stream::Stderr,
                text: text.trim_end_matches('\r').to_owned(),
            })
            .is_ok()
        };
        let mut read = 0;
        if let Ok(bytes) = fs::read(&path) {
            let text = String::from_utf8_lossy(&bytes);
            let all: Vec<&str> = text.lines().collect();
            for line in &all[all.len().saturating_sub(lines)..] {
                if !send(line) {
                    return;
                }
            }
            read = bytes.len() as u64;
        }
        let mut partial = String::new();
        while !stopped.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(250));
            let Ok(mut file) = fs::File::open(&path) else {
                continue;
            };
            let length = file.metadata().map_or(0, |m| m.len());
            if length < read {
                read = 0;
            }
            if length == read || file.seek(SeekFrom::Start(read)).is_err() {
                continue;
            }
            let mut bytes = Vec::new();
            if file.take(length - read).read_to_end(&mut bytes).is_err() {
                continue;
            }
            read = length;
            partial.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(end) = partial.find('\n') {
                let line: String = partial.drain(..=end).collect();
                if !send(line.trim_end_matches('\n')) {
                    return;
                }
            }
        }
    });
    stop
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use super::Service;

    const ID: &str = "0b8f1c2e-5d6a-4b7c-8e9f-0a1b2c3d4e5f";

    #[test]
    fn only_a_uuid_names_a_service() {
        assert!(
            Service::new(ID)
                .unwrap()
                .name()
                .starts_with(&format!("rustbot-{ID}"))
        );
        for bad in ["", "../x", "a b", "ABC", "x.service"] {
            assert!(Service::new(bad).is_none(), "{bad}");
        }
    }

    /// The last lines first, then new ones as they are written, also after a rotation.
    #[test]
    fn a_log_file_is_followed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bot.log");
        std::fs::write(&path, "one\r\ntwo\nthree\n").unwrap();
        let (logs, lines) = std::sync::mpsc::channel();
        let stop = super::follow_file(&path, 2, logs);
        let next = || lines.recv_timeout(Duration::from_secs(5)).unwrap().text;
        assert_eq!((next(), next()), ("two".to_owned(), "three".to_owned()));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, b"four\r\nfi").unwrap();
        assert_eq!(next(), "four");
        std::io::Write::write_all(&mut file, b"ve\n").unwrap();
        assert_eq!(next(), "five");
        drop(file);
        std::fs::write(&path, "new\n").unwrap();
        assert_eq!(next(), "new");
        stop.store(true, Ordering::Relaxed);
    }
}
