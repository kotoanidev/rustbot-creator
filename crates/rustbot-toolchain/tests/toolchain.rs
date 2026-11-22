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

/// Cargo and rustup read these from the project folder, so a project from elsewhere could
/// run code through them.
#[test]
fn build_settings_in_the_project_are_refused() {
    for name in [".cargo", "rust-toolchain", "rust-toolchain.toml"] {
        let dir = tempfile::tempdir().unwrap();
        let project = ping_project(&dir.path().join("bot"), "Pong!");
        std::fs::create_dir_all(project.root.join(name)).unwrap();
        let err = empty_toolchain(dir.path())
            .build(&project, &mut |_| {})
            .unwrap_err();
        assert!(matches!(err, Error::ProjectConfig(_)), "{name}: {err}");
    }
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

    let bot_dir = dir.path().join("bot");
    let project = ping_project(&bot_dir, "Pong!");
    let started = Instant::now();
    let bot = toolchain.build(&project, &mut log).unwrap();
    println!("first build: {:?}", started.elapsed());
    assert!(bot.is_file());
    assert!(bot.starts_with(dir.path().join("data/bots")));

    let project = ping_project(&bot_dir, "Pong, {user.name}!");
    let started = Instant::now();
    toolchain.build(&project, &mut log).unwrap();
    let rebuild = started.elapsed();
    println!("rebuild after a small change: {rebuild:?}");
    assert!(rebuild.as_secs() <= 10, "{rebuild:?}");
}
