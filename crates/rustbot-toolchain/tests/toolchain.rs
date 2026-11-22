//! The private toolchain (D-017, D-062, D-063).

use std::path::Path;
use std::time::Instant;

use rustbot_core::{Graph, GraphFile, GraphKind, Layout, Manifest, Project, Registry};
use rustbot_toolchain::{Error, Toolchain};

/// The `/ping` project, generated and written to `dir`.
fn ping_project(dir: &Path, content: &str) -> Project {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command",
                    "props": { "name": "ping", "description": "Replies with Pong!" } },
            "n2": { "type": "core.reply", "props": { "content": content } }
        },
        "edges": [ { "from": "n1.then", "to": "n2.run" } ]
    }))
    .unwrap();
    let project = if dir.join("project.toml").exists() {
        Project::open(dir).unwrap()
    } else {
        Project::create(dir, Manifest::new("Ping Bot")).unwrap()
    };
    project
        .save_graph(&GraphFile {
            kind: GraphKind::Command,
            name: "ping".to_owned(),
            graph,
            layout: Layout::default(),
        })
        .unwrap();
    let project = Project::open(dir).unwrap();
    let output = rustbot_codegen::generate(&project, &Registry::with_core()).unwrap();
    rustbot_codegen::write(&project, &output).unwrap();
    project
}

fn empty_toolchain(dir: &Path) -> Toolchain {
    Toolchain::in_folders(dir.join("data"), dir.join("cache"))
}

#[test]
fn nothing_is_installed_at_first() {
    let dir = tempfile::tempdir().unwrap();
    let toolchain = empty_toolchain(dir.path());
    assert!(!toolchain.is_installed());
    let project = ping_project(&dir.path().join("bot"), "Pong!");
    assert!(matches!(
        toolchain.build(&project, &mut |_| {}),
        Err(Error::NotInstalled)
    ));
}

/// Installs the real toolchain into a temporary folder, builds `/ping`, then measures a
/// rebuild after a small change (D-038: at most 10 s). Downloads about 300 MB:
/// `cargo test -p rustbot-toolchain -- --ignored install --nocapture`.
#[test]
#[ignore = "downloads rustup, Rust and the bot's crates"]
fn install_build_and_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let toolchain = empty_toolchain(dir.path());
    let mut log = |line: &str| println!("  | {line}");

    let started = Instant::now();
    toolchain.install(&mut log).unwrap();
    assert!(toolchain.is_installed());
    println!("install: {:?}", started.elapsed());

    // The project sits in a repository whose cargo settings would break any build that
    // read them.
    let bot_dir = dir.path().join("repo/bot");
    for folder in ["repo", "repo/bot", "repo/bot/generated"] {
        let config = dir.path().join(folder).join(".cargo");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(
            config.join("config.toml"),
            "[build]\nrustc-wrapper = \"false\"\n",
        )
        .unwrap();
    }
    let project = ping_project(&bot_dir, "Pong!");
    let started = Instant::now();
    let bot = toolchain.build(&project, &mut log).unwrap();
    println!("first build: {:?}", started.elapsed());
    assert!(bot.is_file());
    assert!(bot.starts_with(dir.path().join("data/bots")));

    let project = ping_project(&bot_dir, "Pong, {user.name}!");
    let started = Instant::now();
    let bot = toolchain.build(&project, &mut log).unwrap();
    let rebuild = started.elapsed();
    println!("rebuild after a small change: {rebuild:?}");
    assert!(rebuild.as_secs() <= 10, "{rebuild:?}");

    // The built bot, run as the app runs it, takes the token from its environment and
    // stops with an error when Discord refuses a made-up one; its output reaches the logs.
    let (tx, lines) = std::sync::mpsc::channel();
    let mut running =
        rustbot_runner::Bot::start(&bot, "not.a.real.token", Some("123"), &tx).unwrap();
    let mut output = Vec::new();
    let status = loop {
        output.extend(lines.try_iter().map(|line| line.text));
        if let Some(status) = running.exit_status().unwrap() {
            break status;
        }
        assert!(
            started.elapsed().as_secs() < 120,
            "the bot did not stop: {output:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    std::thread::sleep(std::time::Duration::from_millis(200));
    output.extend(lines.try_iter().map(|line| line.text));
    println!("bot with a made-up token: {status}");
    for line in &output {
        println!("  > {line}");
    }
    assert!(!status.success());
    assert!(output.iter().any(|line| line.contains("401")), "{output:?}");
}
