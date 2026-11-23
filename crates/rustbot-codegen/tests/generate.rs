//! Code generation from a project (D-012, D-057–D-059).

use std::path::Path;
use std::time::Instant;

use proc_macro2::{TokenStream, TokenTree};
use rustbot_codegen::{Error, generate, write};
use rustbot_core::{Graph, GraphFile, GraphKind, Layout, Manifest, Project, Registry};

/// A `/<name>` command replying `content`.
fn command(name: &str, description: &str, content: &str) -> GraphFile {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command",
                    "props": { "name": name, "description": description } },
            "n2": { "type": "core.reply", "props": { "content": content } }
        },
        "edges": [ { "from": "n1.then", "to": "n2.run" } ]
    }))
    .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: name.to_owned(),
        graph,
        layout: Layout::default(),
    }
}

/// `/ban` with an option of every type (D-076) and a reply that reads all of them.
fn ban() -> GraphFile {
    let mut file = command(
        "ban",
        "Bans someone",
        "{option.cel.name} {option.cel.id} {option.cel.username} {option.cel.display_name} \
         {option.cel.mention} | {option.powód} {option.dni} {option.cicho} | \
         {option.kanał.id} {option.kanał.name} {option.kanał.mention} | \
         {option.rola.id} {option.rola.name} {option.rola.mention} {user.mention} @here",
    );
    let options = serde_json::json!([
        { "name": "cel", "type": "user", "description": "Who", "required": true },
        { "name": "powód", "type": "text", "description": "Why" },
        { "name": "dni", "type": "number", "description": "Days of messages to delete" },
        { "name": "cicho", "type": "bool", "description": "Do not announce it" },
        { "name": "kanał", "type": "channel", "description": "Where to log it" },
        { "name": "rola", "type": "role", "description": "A role to mention" }
    ]);
    let trigger = file
        .graph
        .nodes
        .get_mut(&rustbot_core::NodeId("n1".to_owned()))
        .unwrap();
    trigger.props.insert("options".to_owned(), options);
    file
}

/// A project on disk with these graphs, opened again as the app would.
fn project(dir: &Path, manifest: Manifest, graphs: &[GraphFile]) -> Project {
    let project = Project::create(dir, manifest).unwrap();
    for graph in graphs {
        project.save_graph(graph).unwrap();
    }
    Project::open(dir).unwrap()
}

fn ping_project(dir: &Path) -> Project {
    let mut manifest = Manifest::new("Ping Bot");
    manifest.id = "3f2b6c1e-8a4d-4f7e-9b1a-2c5d7e8f9a0b".parse().unwrap();
    project(
        dir,
        manifest,
        &[command("ping", "Replies with Pong!", "Pong, {user.name}!")],
    )
}

#[test]
fn ping_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let output = generate(&ping_project(dir.path()), &Registry::with_core()).unwrap();
    let names: Vec<_> = output
        .files
        .keys()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(
        names,
        [
            "generated/Cargo.toml",
            "generated/rust-toolchain.toml",
            "generated/src/commands/mod.rs",
            "generated/src/commands/ping.rs",
            "generated/src/main.rs",
            "generated/src/settings.rs",
        ]
    );
    for (path, text) in &output.files {
        insta::assert_snapshot!(path.display().to_string(), text);
    }
}

#[test]
fn options_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Ban Bot"), &[ban()]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    insta::assert_snapshot!(
        "options",
        &output.files[Path::new("generated/src/commands/ban.rs")]
    );
}

/// Only the mentions the template's author wrote notify anyone (C-016).
#[test]
fn replies_notify_only_what_the_author_wrote() {
    let reply = |content: &str| {
        let dir = tempfile::tempdir().unwrap();
        let mut file = ban();
        file.graph
            .nodes
            .get_mut(&rustbot_core::NodeId("n2".to_owned()))
            .unwrap()
            .props
            .insert("content".to_owned(), content.into());
        let p = project(dir.path(), Manifest::new("Ban Bot"), &[file]);
        let output = generate(&p, &Registry::with_core()).unwrap();
        output.files[Path::new("generated/src/commands/ban.rs")].replace(char::is_whitespace, "")
    };
    let none = reply("{option.powód} {user.name}");
    assert!(none.contains(".allowed_mentions(djs_rs::model::message::AllowedMentions::default(),"));
    assert!(!none.contains("letmutmentions"));
    let some =
        reply("{user.mention} {option.cel.mention} {option.rola.mention} {option.kanał.mention}");
    assert!(some.contains("mentions.users.push(author.id);"));
    assert!(some.contains(r#"mentions.users.extend(cmd.get_user("cel").map(|user|user.id));"#));
    assert!(some.contains(r#"mentions.roles.extend(cmd.get_role("rola").map(|role|role.id));"#));
    assert!(!some.contains("AllowedMentionType::Everyone"));
    // With @here allowed, a user's "@everyone" in a value must not ping: values go through
    // `quiet`, and only then.
    let here = reply("@here {option.powód}");
    assert!(here.contains("AllowedMentionType::Everyone"));
    assert!(here.contains("quiet(&option_1)"));
    assert!(here.contains("fnquiet("));
    assert!(!none.contains("quiet"));
}

#[test]
fn output_is_deterministic() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let registry = Registry::with_core();
    assert_eq!(
        generate(&ping_project(a.path()), &registry).unwrap(),
        generate(&ping_project(b.path()), &registry).unwrap()
    );
}

/// Every identifier in `tokens`, outside string literals.
fn idents(tokens: TokenStream, out: &mut Vec<String>) {
    for tree in tokens {
        match tree {
            TokenTree::Ident(ident) => out.push(ident.to_string()),
            TokenTree::Group(group) => idents(group.stream(), out),
            _ => {}
        }
    }
}

/// Project content from an untrusted repository must stay data.
#[test]
fn content_cannot_inject_code() {
    let evil = r#"x"); std::process::exit(1); ("{{}} \" \\ {"#;
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.bot.package = "bot\"\n[dependencies]\nevil = \"1\"".to_owned();
    let project = project(
        dir.path(),
        manifest,
        &[command(
            "ping",
            evil,
            &format!(
                "{} {{user.name}}",
                evil.replace('{', "{{").replace('}', "}}")
            ),
        )],
    );
    let output = generate(&project, &Registry::with_core()).unwrap();

    let code = &output.files[Path::new("generated/src/commands/ping.rs")];
    let tokens: TokenStream = code.parse().unwrap();
    let mut found = Vec::new();
    idents(tokens, &mut found);
    assert!(!found.contains(&"exit".to_owned()), "{code}");

    let cargo: toml::Table = output.files[Path::new("generated/Cargo.toml")]
        .parse()
        .unwrap();
    assert_eq!(
        cargo["package"]["name"].as_str(),
        Some(project.manifest.bot.package.as_str())
    );
    assert_eq!(cargo["dependencies"].as_table().unwrap().len(), 3);
}

#[test]
fn literal_braces_stay_literal() {
    let dir = tempfile::tempdir().unwrap();
    let project = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "{{user.name}} is {user.name}")],
    );
    let output = generate(&project, &Registry::with_core()).unwrap();
    let code = &output.files[Path::new("generated/src/commands/ping.rs")];
    assert!(
        code.contains(r#"format!("{{user.name}} is {}", user_name)"#),
        "{code}"
    );
}

#[test]
fn errors() {
    let registry = Registry::with_core();

    let dir = tempfile::tempdir().unwrap();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "{usr}")],
    );
    assert!(matches!(
        generate(&p, &registry),
        Err(Error::Invalid { .. })
    ));

    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.gateway.extra_intents = vec!["EVERYTHING".to_owned()];
    let p = project(dir.path(), manifest, &[command("ping", "d", "hi")]);
    assert!(matches!(generate(&p, &registry), Err(Error::UnknownIntent(i)) if i == "EVERYTHING"));

    let dir = tempfile::tempdir().unwrap();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("a-b", "d", "hi"), command("a_b", "d", "hi")],
    );
    assert!(matches!(generate(&p, &registry), Err(Error::NameClash(..))));

    let dir = tempfile::tempdir().unwrap();
    let mut looped = command("ping", "d", "hi");
    looped.graph.edges.push(rustbot_core::Edge {
        from: "n2.then".parse().unwrap(),
        to: "n2.run".parse().unwrap(),
    });
    let p = project(dir.path(), Manifest::new("Bot"), &[looped]);
    assert!(matches!(generate(&p, &registry), Err(Error::Cycle { .. })));
}

#[test]
fn intents_are_written_from_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.gateway.extra_intents = vec!["MESSAGE_CONTENT".to_owned(), "GUILDS".to_owned()];
    let p = project(dir.path(), manifest, &[command("ping", "d", "hi")]);
    let output = generate(&p, &Registry::with_core()).unwrap();
    let main = &output.files[Path::new("generated/src/main.rs")];
    assert!(
        main.contains("Intents::GUILDS | Intents::MESSAGE_CONTENT"),
        "{main}"
    );
}

#[test]
fn write_removes_stale_modules() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Registry::with_core();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "hi"), command("old", "d", "hi")],
    );
    write(&p, &generate(&p, &registry).unwrap()).unwrap();
    assert!(dir.path().join("generated/src/commands/old.rs").exists());

    std::fs::remove_file(dir.path().join("graphs/commands/old.json")).unwrap();
    let p = Project::open(dir.path()).unwrap();
    write(&p, &generate(&p, &registry).unwrap()).unwrap();
    assert!(!dir.path().join("generated/src/commands/old.rs").exists());
    assert!(dir.path().join("generated/src/commands/ping.rs").exists());
}

/// The generated `/ping` bot compiles against the real djs-rs from crates.io. Needs the
/// network the first time: `cargo test -p rustbot-codegen -- --ignored compiles`.
#[test]
#[ignore = "downloads and builds djs-rs"]
fn generated_bot_compiles() {
    let dir = tempfile::tempdir().unwrap();
    let mut graphs = vec![command(
        "ping",
        "Replies with Pong!",
        "Pong, {user.name}! {user.id} {user.username} {user.display_name} {user.mention}",
    )];
    graphs.push(command(
        "where",
        "Says where it is",
        "{channel.id} {channel.name} {channel.mention} {server.id} {server.name}",
    ));
    graphs.push(ban());
    let p = project(dir.path(), Manifest::new("Ping Bot"), &graphs);
    write(&p, &generate(&p, &Registry::with_core()).unwrap()).unwrap();

    // One target folder for every run, so djs-rs is built only once.
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated-bots");
    let status = std::process::Command::new(env!("CARGO"))
        .args(["clippy", "--quiet", "--", "-D", "warnings"])
        .current_dir(dir.path().join("generated"))
        .env("CARGO_TARGET_DIR", target)
        .status()
        .unwrap();
    assert!(status.success());
}

/// D-038: generating 500 nodes takes at most 100 ms. Measured in release:
/// `cargo test -p rustbot-codegen --release -- --ignored budget --nocapture`.
#[test]
#[ignore = "a measurement, run in release"]
fn budget_500_nodes() {
    let mut file = command("big", "d", "{user.name}");
    for i in 3..=501 {
        let node = file.graph.nodes[&rustbot_core::NodeId("n2".to_owned())].clone();
        file.graph
            .nodes
            .insert(rustbot_core::NodeId(format!("n{i}")), node);
        file.graph.edges.push(rustbot_core::Edge {
            from: format!("n{}.then", i - 1).parse().unwrap(),
            to: format!("n{i}.run").parse().unwrap(),
        });
    }
    assert_eq!(file.graph.nodes.len(), 501);
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Bot"), &[file]);
    let registry = Registry::with_core();

    let runs = 20;
    let started = Instant::now();
    for _ in 0..runs {
        generate(&p, &registry).unwrap();
    }
    let each = started.elapsed() / runs;
    println!("generate, 501 nodes: {each:?} per run");
    assert!(each.as_millis() <= 100, "{each:?}");
}

/// Files a cloned project hides in `generated/` would run at build time; they are removed.
#[test]
fn write_owns_all_of_generated() {
    let dir = tempfile::tempdir().unwrap();
    let p = ping_project(dir.path());
    for planted in [
        "generated/build.rs",
        "generated/.cargo/config.toml",
        "generated/examples/x.rs",
    ] {
        p.write_file(Path::new(planted), "planted").unwrap();
    }
    p.write_file(Path::new("generated/Cargo.lock"), "lock")
        .unwrap();
    p.write_file(Path::new("generated/target/debug/x"), "build output")
        .unwrap();

    write(&p, &generate(&p, &Registry::with_core()).unwrap()).unwrap();
    let left: Vec<String> = p
        .files_under(Path::new("generated"), &[])
        .unwrap()
        .iter()
        .map(|f| f.display().to_string())
        .collect();
    assert_eq!(
        left,
        [
            "generated/Cargo.lock",
            "generated/Cargo.toml",
            "generated/rust-toolchain.toml",
            "generated/src/commands/mod.rs",
            "generated/src/commands/ping.rs",
            "generated/src/main.rs",
            "generated/src/settings.rs",
            "generated/target/debug/x",
        ]
    );
}
