//! Running a bot, with shell scripts standing in for built bots.

#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::sync::mpsc;
#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use rustbot_runner::{Bot, LogLine, Stream};
use rustbot_runner::{Error, set_test_server, test_server};

#[cfg(unix)]
fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Collects lines until every one of `lines` has arrived; stdout and stderr interleave freely.
#[cfg(unix)]
fn wait_for(logs: &mpsc::Receiver<LogLine>, lines: &[(Stream, &str)]) {
    let mut seen = Vec::new();
    while !lines.iter().all(|&(stream, text)| {
        seen.iter()
            .any(|l: &LogLine| l.stream == stream && l.text == text)
    }) {
        seen.push(logs.recv_timeout(Duration::from_secs(10)).unwrap());
    }
}

/// One test, so no other test thread forks while a script is being written (ETXTBSY).
#[cfg(unix)]
#[test]
fn start_log_and_stop() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, logs) = mpsc::channel();

    // Environment, both streams, and a clean stop on SIGINT.
    let bot = script(
        dir.path(),
        "polite",
        r#"trap 'echo bye; exit 0' INT
echo "token=$DISCORD_TOKEN guild=${DISCORD_TEST_GUILD_ID:-none} no_color=$NO_COLOR dir=$(pwd)"
echo oops >&2
echo ready
while :; do sleep 0.05; done"#,
    );
    let mut running = Bot::start(&bot, " t0k3n \n", Some(" 123456789012345678 "), &tx).unwrap();
    let env = format!(
        "token=t0k3n guild=123456789012345678 no_color=1 dir={}",
        std::fs::canonicalize(dir.path()).unwrap().display()
    );
    wait_for(
        &logs,
        &[
            (Stream::Stdout, &env),
            (Stream::Stderr, "oops"),
            (Stream::Stdout, "ready"),
        ],
    );
    assert_eq!(running.exit_status().unwrap(), None);
    assert!(running.stop(Duration::from_secs(5)).unwrap().success());
    wait_for(&logs, &[(Stream::Stdout, "bye")]);
    // Stopping again reports the same exit.
    assert!(running.stop(Duration::from_secs(5)).unwrap().success());

    // No test server: an inherited one is not passed on.
    let quiet = script(
        dir.path(),
        "quiet",
        r#"echo "guild=${DISCORD_TEST_GUILD_ID:-none}""#,
    );
    let mut done = Bot::start(&quiet, "t", None, &tx).unwrap();
    wait_for(&logs, &[(Stream::Stdout, "guild=none")]);
    while done.exit_status().unwrap().is_none() {}

    // A bot that ignores SIGINT is killed after the grace period.
    let stubborn = script(
        dir.path(),
        "stubborn",
        "trap '' INT\necho ready\nwhile :; do sleep 0.05; done",
    );
    let mut running = Bot::start(&stubborn, "t", None, &tx).unwrap();
    wait_for(&logs, &[(Stream::Stdout, "ready")]);
    let started = Instant::now();
    assert!(!running.stop(Duration::from_millis(300)).unwrap().success());
    assert!(started.elapsed() >= Duration::from_millis(300));

    assert!(matches!(
        Bot::start(&quiet, "  ", None, &tx),
        Err(Error::NoToken)
    ));
    assert!(matches!(
        Bot::start(&quiet, "t", Some("12a"), &tx),
        Err(Error::InvalidServerId(_))
    ));
}

#[test]
fn test_server_is_stored_per_bot() {
    let dir = tempfile::tempdir().unwrap();
    let bot_dir = dir.path().join("bots/some-id");
    assert_eq!(test_server(&bot_dir).unwrap(), None);
    set_test_server(&bot_dir, Some(" 987654321 ")).unwrap();
    assert_eq!(test_server(&bot_dir).unwrap().as_deref(), Some("987654321"));
    for bad in ["", "abc", "-1", "0", "99999999999999999999999"] {
        assert!(matches!(
            set_test_server(&bot_dir, Some(bad)),
            Err(Error::InvalidServerId(_))
        ));
    }
    set_test_server(&bot_dir, None).unwrap();
    set_test_server(&bot_dir, None).unwrap();
    assert_eq!(test_server(&bot_dir).unwrap(), None);
}
