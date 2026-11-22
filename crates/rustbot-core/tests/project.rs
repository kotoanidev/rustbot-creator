//! The project format v0 (D-051) on disk.

use std::fs;

use rustbot_core::{Error, Graph, GraphFile, GraphKind, Layout, Manifest, Project};

/// The `/ping` graph from D-051.
fn ping() -> GraphFile {
    let graph: Graph = serde_json::from_str(
        r#"{ "schema": 1,
             "nodes": {
               "n1": { "type": "core.slash_command",
                       "props": { "name": "ping", "description": "Replies with Pong!" } },
               "n2": { "type": "core.reply", "props": { "content": "Pong, {user.name}!" } } },
             "edges": [ { "from": "n1.then", "to": "n2.run" } ] }"#,
    )
    .unwrap();
    let layout: Layout =
        serde_json::from_str(r#"{ "schema": 1, "nodes": { "n1": [0, 0], "n2": [280, 0] } }"#)
            .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: "ping".to_owned(),
        graph,
        layout,
    }
}

#[test]
fn create_save_open_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Ping Bot")).unwrap();
    project.save_graph(&ping()).unwrap();

    let opened = Project::open(dir.path()).unwrap();
    assert_eq!(opened.manifest, project.manifest);
    assert_eq!(opened.manifest.bot.package, "ping-bot");
    assert_eq!(opened.graphs, vec![ping()]);
    assert_eq!(
        fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        "target/\n.env\n"
    );
}

#[test]
fn graph_files_are_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Ping Bot")).unwrap();
    project.save_graph(&ping()).unwrap();

    // Keys are sorted (`description` before `name`) whatever the input order was.
    let expected = r#"{
  "schema": 1,
  "nodes": {
    "n1": {
      "type": "core.slash_command",
      "props": {
        "description": "Replies with Pong!",
        "name": "ping"
      }
    },
    "n2": {
      "type": "core.reply",
      "props": {
        "content": "Pong, {user.name}!"
      }
    }
  },
  "edges": [
    {
      "from": "n1.then",
      "to": "n2.run"
    }
  ]
}
"#;
    let path = dir.path().join("graphs/commands/ping.json");
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);

    // Saving what was opened writes the same bytes.
    let opened = Project::open(dir.path()).unwrap();
    opened.save_graph(&opened.graphs[0]).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
}

#[test]
fn manifest_file() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Ping Bot")).unwrap();
    let text = fs::read_to_string(dir.path().join("project.toml")).unwrap();
    let expected = format!(
        "schema = 1\nid = \"{}\"\n\n[bot]\nname = \"Ping Bot\"\npackage = \"ping-bot\"\n\
         description = \"\"\nlicense = \"\"\n\n[djs-rs]\nversion = \"0.1.0-alpha.1\"\n\
         features = []\n\n[gateway]\nextra-intents = []\n",
        project.manifest.id
    );
    assert_eq!(text, expected);
}

#[test]
fn create_never_overwrites_a_project() {
    let dir = tempfile::tempdir().unwrap();
    Project::create(dir.path(), Manifest::new("First")).unwrap();
    let err = Project::create(dir.path(), Manifest::new("Second")).unwrap_err();
    assert!(matches!(err, Error::AlreadyExists(_)), "{err}");
    let kept = Project::open(dir.path()).unwrap();
    assert_eq!(kept.manifest.bot.name, "First");
}

#[test]
fn newer_format_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
    let path = dir.path().join("project.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("schema = 1", "schema = 2");
    fs::write(&path, text).unwrap();
    let err = Project::open(&project.root).unwrap_err();
    assert!(
        matches!(err, Error::UnsupportedSchema { found: 2, .. }),
        "{err}"
    );
}

#[test]
fn unknown_settings_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    Project::create(dir.path(), Manifest::new("Bot")).unwrap();
    let path = dir.path().join("project.toml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("[bot]", "[bot]\nnmae = \"typo\"");
    fs::write(&path, text).unwrap();
    let err = Project::open(dir.path()).unwrap_err();
    assert!(matches!(err, Error::Toml { .. }), "{err}");
}

#[test]
fn graph_names_cannot_escape_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
    for name in ["../evil", "a/b", ".hidden", "x.layout", ""] {
        let file = GraphFile {
            name: name.to_owned(),
            ..ping()
        };
        let err = project.save_graph(&file).unwrap_err();
        assert!(matches!(err, Error::InvalidGraphName(_)), "{name:?}: {err}");
    }
}

/// A cloned repository can contain symlinks; none of them may redirect a write or a read.
#[cfg(unix)]
mod symlinks {
    use std::fs;
    use std::os::unix::fs::symlink;

    use super::ping;
    use rustbot_core::{Error, Manifest, Project};

    #[test]
    fn a_planted_temporary_file_cannot_redirect_a_write() {
        let outside = tempfile::tempdir().unwrap();
        let victim = outside.path().join("victim");
        fs::write(&victim, "keep me").unwrap();

        let dir = tempfile::tempdir().unwrap();
        let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
        let planted = dir
            .path()
            .join(format!(".project.toml.{}.tmp", std::process::id()));
        symlink(&victim, &planted).unwrap();

        project.save_manifest().unwrap();
        assert_eq!(fs::read_to_string(&victim).unwrap(), "keep me");
        assert!(!planted.exists());
        Project::open(dir.path()).unwrap();
    }

    #[test]
    fn a_symlinked_graphs_folder_is_refused() {
        let outside = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
        symlink(outside.path(), dir.path().join("graphs")).unwrap();

        let err = project.save_graph(&ping()).unwrap_err();
        assert!(matches!(err, Error::Symlink(_)), "{err}");
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        let err = Project::open(dir.path()).unwrap_err();
        assert!(matches!(err, Error::Symlink(_)), "{err}");
    }

    #[test]
    fn a_symlinked_generated_folder_is_refused() {
        use std::path::Path;

        let outside = tempfile::tempdir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
        symlink(outside.path(), dir.path().join("generated")).unwrap();
        let err = project
            .write_file(Path::new("generated/src/main.rs"), "")
            .unwrap_err();
        assert!(matches!(err, Error::Symlink(_)), "{err}");
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_symlinked_graph_file_is_not_read() {
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret.json");
        fs::write(&secret, r#"{ "schema": 1 }"#).unwrap();

        let dir = tempfile::tempdir().unwrap();
        Project::create(dir.path(), Manifest::new("Bot")).unwrap();
        let commands = dir.path().join("graphs/commands");
        fs::create_dir_all(&commands).unwrap();
        symlink(&secret, commands.join("leak.json")).unwrap();

        let err = Project::open(dir.path()).unwrap_err();
        assert!(matches!(err, Error::Symlink(_)), "{err}");
    }
}

#[test]
fn other_files_stay_inside_the_project() {
    use std::path::Path;

    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(dir.path(), Manifest::new("Bot")).unwrap();
    project
        .write_file(Path::new("generated/src/main.rs"), "fn main() {}\n")
        .unwrap();
    project
        .write_file(Path::new("generated/Cargo.toml"), "")
        .unwrap();
    assert_eq!(
        project.files_under(Path::new("generated"), &[]).unwrap(),
        vec![
            Path::new("generated/Cargo.toml").to_owned(),
            Path::new("generated/src/main.rs").to_owned(),
        ]
    );
    project
        .remove_file(Path::new("generated/src/main.rs"))
        .unwrap();
    project
        .remove_file(Path::new("generated/src/main.rs"))
        .unwrap();
    assert_eq!(
        project.files_under(Path::new("nothing"), &[]).unwrap(),
        Vec::<std::path::PathBuf>::new()
    );

    for bad in ["../x", "/etc/passwd", "generated/../../x"] {
        let err = project.write_file(Path::new(bad), "").unwrap_err();
        assert!(matches!(err, Error::InvalidPath(_)), "{bad}: {err}");
    }
}
