//! Export (D-134–D-137): the bot as a standalone Cargo project, or as a ready build, in a
//! new folder. Both come with a README, `.env.example` and the bot's license (D-023).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use gpui_kit::{App, AppContext as _, AsyncWindowContext, Entity, PromptLevel, Window};
use rustbot_core::Project;

use crate::i18n::{t, t_with};
use crate::install::{ask, install, logged};
use crate::run::{self, Run};
use crate::state::AppState;

/// Marks a folder as an export, so a later export may replace it.
const MARKER: &str = ".rustbot-export";

/// The licenses offered in the bot settings (D-136), as SPDX expressions; empty is none.
pub(crate) const LICENSES: [&str; 6] = [
    "",
    "MIT",
    "Apache-2.0",
    "MIT OR Apache-2.0",
    "MIT-0",
    "GPL-3.0-or-later",
];

const MIT: &str = include_str!("../licenses/MIT.txt");
const MIT_0: &str = include_str!("../licenses/MIT-0.txt");
const APACHE: &str = include_str!("../licenses/Apache-2.0.txt");
const GPL: &str = include_str!("../licenses/GPL-3.0.txt");

const ENV_EXAMPLE: &str = "\
# The bot's token, from the \"Bot\" page of the Discord Developer Portal.
# Keep it secret: never share this file or commit it.
DISCORD_TOKEN=
# A server ID, to register the slash commands only there while testing.
# Empty: they are registered on every server the bot is on.
DISCORD_TEST_GUILD_ID=
";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Source,
    Build,
}

/// Asks what to export and where, then exports, with the progress in the Logs panel.
pub(crate) fn start(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    if !matches!(state.read(cx).run, Run::Stopped) {
        return;
    }
    let package = state.read(cx).project.manifest.bot.package.clone();
    window
        .spawn(cx, async move |cx| {
            let Some(kind) = choose(cx).await else { return };
            let Some(target) = folder(cx, &package, kind).await else {
                return;
            };
            // Run may have been pressed while the dialogs were open.
            let started = state.update(cx, |state, cx| {
                let stopped = matches!(state.run, Run::Stopped);
                if stopped {
                    state.run = Run::Busy(t("export-busy"));
                    cx.notify();
                }
                stopped
            });
            if !started {
                return;
            }
            let result = export(&state, kind, &target, cx).await;
            state.update(cx, |state, cx| {
                match result {
                    Ok(()) => state.log(t_with(
                        "export-done",
                        &[("path", &target.display().to_string())],
                    )),
                    Err(message) => state.log_error(message),
                }
                state.run = Run::Stopped;
                cx.notify();
            });
        })
        .detach();
}

async fn choose(cx: &mut AsyncWindowContext) -> Option<Kind> {
    let (title, detail) = (t("export-title"), t("export-detail"));
    let answers = [t("export-source"), t("export-build"), t("cancel")];
    let answers = answers.each_ref().map(String::as_str);
    let answer = cx
        .update(|window, cx| window.prompt(PromptLevel::Info, &title, Some(&detail), &answers, cx))
        .ok()?;
    match answer.await {
        Ok(0) => Some(Kind::Source),
        Ok(1) => Some(Kind::Build),
        _ => None,
    }
}

async fn folder(cx: &mut AsyncWindowContext, package: &str, kind: Kind) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    let name = match kind {
        Kind::Source => package.to_owned(),
        Kind::Build => format!("{package}-linux-{}", std::env::consts::ARCH),
    };
    let path = cx
        .update(|_, cx| cx.prompt_for_new_path(&home, Some(&name)))
        .ok()?;
    path.await.ok()?.ok().flatten()
}

async fn export(
    state: &Entity<AppState>,
    kind: Kind,
    target: &Path,
    cx: &mut AsyncWindowContext,
) -> Result<(), String> {
    let failed = |e: io::Error| t_with("error-export", &[("error", &e.to_string())]);
    let path = target.display().to_string();
    // Asked before anything is built, so a "no" wastes nothing.
    let replace = match existing(target).map_err(failed)? {
        Existing::Free => false,
        Existing::Export => {
            let detail = t_with("export-replace-detail", &[("path", &path)]);
            if !ask(
                cx,
                &t("export-replace-title"),
                &detail,
                &t("export-replace"),
            )
            .await
            {
                return Err(t("export-cancelled"));
            }
            true
        }
        Existing::Other => return Err(t_with("error-export-exists", &[("path", &path)])),
    };
    let (project, binary) = match kind {
        Kind::Source => {
            let project = run::checked(state, cx)?;
            (project, None)
        }
        Kind::Build => {
            let (project, toolchain) = run::check(state, cx).await?;
            let report = run::report(state);
            install(cx, &toolchain, &report).await?;
            run::generate(state, cx)?;
            run::busy(state, cx, &t("export-building"));
            let built = project.clone();
            let binary = logged(cx, &report, move |log| toolchain.build_release(&built, log))
                .await
                .map_err(|e| t_with("error-build", &[("error", &e.to_string())]))?;
            (project, Some(binary))
        }
    };
    let files = match kind {
        Kind::Source => source_files(&project, run::generate(state, cx)?),
        Kind::Build => extras(&project, Kind::Build),
    };
    run::busy(state, cx, &t("export-writing"));
    let target = target.to_owned();
    let package = project.manifest.bot.package.clone();
    cx.background_spawn(async move {
        let binary = binary.as_deref().map(|path| (path, package.as_str()));
        place(&target, &files, binary, replace)
    })
    .await
    .map_err(failed)
}

/// What is already at an export's folder.
#[derive(Debug, PartialEq, Eq)]
enum Existing {
    /// Nothing, or an empty folder.
    Free,
    /// An earlier export, which may be replaced.
    Export,
    /// Something else, which is never touched.
    Other,
}

fn existing(target: &Path) -> io::Result<Existing> {
    let metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Existing::Free),
        Err(e) => return Err(e),
    };
    if !metadata.is_dir() {
        return Ok(Existing::Other);
    }
    if fs::read_dir(target)?.next().is_none() {
        Ok(Existing::Free)
    } else if fs::symlink_metadata(target.join(MARKER)).is_ok_and(|m| m.is_file()) {
        Ok(Existing::Export)
    } else {
        Ok(Existing::Other)
    }
}

/// The source export: `generated/` as the project's root, its `Cargo.lock` when there is
/// one, a `.gitignore`, and the extras.
fn source_files(project: &Project, output: rustbot_codegen::Output) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files: BTreeMap<PathBuf, Vec<u8>> = output
        .files
        .into_iter()
        .filter_map(|(path, text)| {
            let path = path.strip_prefix("generated").ok()?.to_owned();
            Some((path, text.into_bytes()))
        })
        .collect();
    // The versions the bot was built and tried with. Only a plain file: a project from
    // elsewhere could link it to a secret.
    let lock = project.root.join("generated").join("Cargo.lock");
    if fs::symlink_metadata(&lock).is_ok_and(|m| m.is_file())
        && let Ok(text) = fs::read(&lock)
    {
        files.insert("Cargo.lock".into(), text);
    }
    files.insert(".gitignore".into(), b"target/\n.env\n".to_vec());
    files.extend(extras(project, Kind::Source));
    files
}

/// The files both exports have: the README, `.env.example` and the license.
fn extras(project: &Project, kind: Kind) -> BTreeMap<PathBuf, Vec<u8>> {
    let bot = &project.manifest.bot;
    let mut files = BTreeMap::new();
    files.insert(".env.example".into(), ENV_EXAMPLE.as_bytes().to_vec());
    files.insert("README.md".into(), readme(bot, kind).into_bytes());
    for (name, text) in license_files(bot) {
        files.insert(name.into(), text.into_bytes());
    }
    files
}

fn readme(bot: &rustbot_core::BotSettings, kind: Kind) -> String {
    let mut text = format!("# {}\n\n", bot.name);
    if !bot.description.is_empty() {
        let _ = write!(text, "{}\n\n", bot.description);
    }
    match kind {
        Kind::Source => text.push_str(
            "A Discord bot made with [RustBot Creator](https://github.com/kotoanidev/rustbot-creator) \
             on [djs-rs](https://github.com/kotoanidev/djs-rs). This folder is a plain Rust project: \
             it builds without RustBot Creator, which does not read changes made here.\n\n\
             ## Build and run\n\n\
             1. Install Rust with [rustup](https://rustup.rs). `rust-toolchain.toml` picks the version.\n\
             2. Install a C compiler, e.g. `gcc` (`sudo dnf install gcc` or `sudo apt install gcc`).\n\
             3. Copy `.env.example` to `.env` and put the bot's token in it.\n\
             4. Run `cargo run --release`.\n\n",
        ),
        Kind::Build => {
            let _ = write!(
                text,
            "A Discord bot made with [RustBot Creator](https://github.com/kotoanidev/rustbot-creator), \
             built for Linux ({arch}).\n\n\
             ## Run\n\n\
             1. Copy `.env.example` to `.env` and put the bot's token in it.\n\
             2. Run `./{package}` in this folder.\n\n",
            arch = std::env::consts::ARCH,
            package = bot.package,
        );
        }
    }
    text.push_str(
        "The bot registers its slash commands on every server it is on. To try changes on one \
         server first, set `DISCORD_TEST_GUILD_ID` in `.env`.\n",
    );
    if !bot.license.is_empty() {
        let _ = write!(text, "\n## License\n\n{}\n", bot.license);
    }
    text
}

/// The license's files, with the bot's authors as the copyright holder where the text
/// names one.
fn license_files(bot: &rustbot_core::BotSettings) -> Vec<(&'static str, String)> {
    let holder = format!("The authors of {}", bot.name);
    let fill = |text: &str| text.replace("{holder}", &holder);
    match bot.license.as_str() {
        "MIT" => vec![("LICENSE", fill(MIT))],
        "MIT-0" => vec![("LICENSE", fill(MIT_0))],
        "Apache-2.0" => vec![("LICENSE", APACHE.to_owned())],
        "MIT OR Apache-2.0" => vec![
            ("LICENSE-MIT", fill(MIT)),
            ("LICENSE-APACHE", APACHE.to_owned()),
        ],
        "GPL-3.0-or-later" | "GPL-3.0-only" => vec![("LICENSE", GPL.to_owned())],
        _ => Vec::new(),
    }
}

/// Writes the export next to `target`, then puts it in its place: into a free spot, or,
/// when `replace`, over an earlier export. Nothing else is ever overwritten.
fn place(
    target: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    binary: Option<(&Path, &str)>,
    replace: bool,
) -> io::Result<()> {
    let refused = |what: &str| io::Error::new(io::ErrorKind::AlreadyExists, what.to_owned());
    let (Some(parent), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(refused("not a folder to export to"));
    };
    let name = name.to_string_lossy();
    let new = parent.join(format!(".{name}.export"));
    let old = parent.join(format!(".{name}.old"));
    for leftover in [&new, &old] {
        match existing(leftover)? {
            Existing::Free => {
                let _ = fs::remove_dir(leftover);
            }
            Existing::Export => fs::remove_dir_all(leftover)?,
            Existing::Other => return Err(refused(&leftover.display().to_string())),
        }
    }
    fs::create_dir_all(&new)?;
    // The marker first, so a half-written export is still known as one.
    fs::write(new.join(MARKER), "Made by RustBot Creator's export.\n")?;
    for (path, data) in files {
        let path = new.join(path);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, data)?;
    }
    if let Some((from, file)) = binary {
        fs::copy(from, new.join(file))?;
    }
    match existing(target)? {
        Existing::Free => {
            let _ = fs::remove_dir(target);
            fs::rename(&new, target)
        }
        Existing::Export if replace => {
            fs::rename(target, &old)?;
            fs::rename(&new, target)?;
            fs::remove_dir_all(&old)
        }
        Existing::Export | Existing::Other => {
            fs::remove_dir_all(&new)?;
            Err(refused(&target.display().to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Existing, Kind};

    fn project(license: &str) -> (tempfile::TempDir, rustbot_core::Project) {
        let dir = tempfile::tempdir().unwrap();
        let template = &crate::templates::TEMPLATES[0];
        let mut project = crate::templates::create(&dir.path().join("bot"), template).unwrap();
        project.manifest.bot.license = license.to_owned();
        (dir, project)
    }

    /// The source export is `generated/` as a project of its own, with the extras, and
    /// replaces only an earlier export.
    #[test]
    fn the_source_export_is_a_cargo_project() {
        let (dir, project) = project("MIT OR Apache-2.0");
        let output =
            rustbot_codegen::generate(&project, &rustbot_core::Registry::with_core()).unwrap();
        let files = super::source_files(&project, output);
        let target = dir.path().join("out");
        super::place(&target, &files, None, false).unwrap();
        for file in [
            "Cargo.toml",
            "rust-toolchain.toml",
            "src/main.rs",
            "README.md",
            ".env.example",
            ".gitignore",
            "LICENSE-MIT",
            "LICENSE-APACHE",
            super::MARKER,
        ] {
            assert!(target.join(file).is_file(), "{file}");
        }
        assert!(!target.join("generated").exists());
        let mit = std::fs::read_to_string(target.join("LICENSE-MIT")).unwrap();
        assert!(mit.contains(&format!(
            "Copyright (c) The authors of {}",
            project.manifest.bot.name
        )));

        // An earlier export is replaced only when asked; other folders never are.
        assert_eq!(super::existing(&target).unwrap(), Existing::Export);
        assert!(super::place(&target, &files, None, false).is_err());
        std::fs::write(target.join("stale.txt"), "old").unwrap();
        super::place(&target, &files, None, true).unwrap();
        assert!(!target.join("stale.txt").exists());
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(other.join("mine.txt"), "keep").unwrap();
        assert_eq!(super::existing(&other).unwrap(), Existing::Other);
        assert!(super::place(&other, &files, None, true).is_err());
        assert!(other.join("mine.txt").is_file());
        assert!(!dir.path().join(".other.export").exists());
    }

    #[test]
    fn the_build_export_has_the_program_and_its_settings() {
        let (dir, project) = project("");
        let program = dir.path().join("program");
        std::fs::write(&program, "binary").unwrap();
        let files = super::extras(&project, Kind::Build);
        let target = dir.path().join("build");
        let package = &project.manifest.bot.package;
        super::place(&target, &files, Some((&program, package)), false).unwrap();
        assert!(target.join(package).is_file());
        assert!(!target.join("LICENSE").exists());
        let readme = std::fs::read_to_string(target.join("README.md")).unwrap();
        assert!(readme.contains(&format!("Run `./{package}`")));
        assert!(
            std::fs::read_to_string(target.join(".env.example"))
                .unwrap()
                .contains("DISCORD_TOKEN=")
        );
        assert!(Path::new(&target).join(super::MARKER).is_file());
    }

    /// Every license on offer has its text in the export, except none.
    #[test]
    fn every_offered_license_has_its_text() {
        for license in super::LICENSES {
            let (_dir, project) = project(license);
            let files = super::license_files(&project.manifest.bot);
            assert_eq!(files.is_empty(), license.is_empty(), "{license}");
            assert!(files.iter().all(|(_, text)| !text.contains("{holder}")));
        }
    }
}
