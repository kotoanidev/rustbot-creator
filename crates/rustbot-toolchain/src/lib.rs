//! The private Rust toolchain RustBot Creator installs and builds bots with (D-017, D-039,
//! D-061–D-063).
//!
//! rustup and Rust [`RUST_VERSION`] live in the app's data folder and never touch the
//! user's `~/.cargo` or `~/.rustup`. Every bot builds into one shared target folder, and
//! the binary is then copied into a folder of its own.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rustbot_core::{Project, RUST_VERSION};
use sha2::{Digest, Sha256};

/// Why installing or building failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file or folder could not be used.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The home folder is unknown, so there is nowhere to install.
    #[error("the home folder is unknown")]
    NoHome,
    /// This computer's platform has no rustup build.
    #[error("no Rust toolchain for {0}")]
    UnsupportedPlatform(String),
    /// A download failed.
    #[error("downloading {url}: {message}")]
    Download {
        /// What was downloaded.
        url: String,
        /// Why it failed.
        message: String,
    },
    /// A download does not match its published checksum.
    #[error("{url} does not match its checksum")]
    Checksum {
        /// What was downloaded.
        url: String,
    },
    /// A program exited with an error.
    #[error("{program} failed ({status})")]
    Failed {
        /// What ran.
        program: String,
        /// How it exited.
        status: std::process::ExitStatus,
    },
    /// The toolchain is not installed yet.
    #[error("the Rust toolchain is not installed")]
    NotInstalled,
    /// The project could not be written or read.
    #[error(transparent)]
    Project(#[from] rustbot_core::Error),
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}

/// The app's folders (D-061): data in `~/.local/share/rustbot-creator`, caches in
/// `~/.cache/rustbot-creator`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    data: PathBuf,
    cache: PathBuf,
}

impl Toolchain {
    /// The toolchain in the user's XDG folders.
    ///
    /// # Errors
    /// [`Error::NoHome`] when the home folder is unknown.
    pub fn locate() -> Result<Self, Error> {
        let dirs = directories::BaseDirs::new().ok_or(Error::NoHome)?;
        Ok(Self::in_folders(
            dirs.data_dir().join("rustbot-creator"),
            dirs.cache_dir().join("rustbot-creator"),
        ))
    }

    /// A toolchain in the given data and cache folders, e.g. for tests.
    #[must_use]
    pub fn in_folders(data: PathBuf, cache: PathBuf) -> Self {
        Self { data, cache }
    }

    fn rustup_home(&self) -> PathBuf {
        self.data.join("rustup")
    }

    fn cargo_home(&self) -> PathBuf {
        self.data.join("cargo")
    }

    fn bin(&self, program: &str) -> PathBuf {
        self.cargo_home()
            .join("bin")
            .join(format!("{program}{}", std::env::consts::EXE_SUFFIX))
    }

    /// The app's data folder.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    /// The shared target folder of every bot (D-063).
    #[must_use]
    pub fn target_dir(&self) -> PathBuf {
        self.cache.join("target")
    }

    /// The project's folder in the app's data: the built bot and the user's own settings
    /// for it, such as the test server (D-065).
    #[must_use]
    pub fn bot_dir(&self, project: &Project) -> PathBuf {
        self.bot_dir_by_id(&project.manifest.id.to_string())
    }

    /// [`Toolchain::bot_dir`] by the project's id alone, for the hosted bot's launcher
    /// (D-138). The caller checks that `id` is a UUID.
    #[must_use]
    pub fn bot_dir_by_id(&self, id: &str) -> PathBuf {
        self.data.join("bots").join(id)
    }

    /// Whether Rust [`RUST_VERSION`] is installed.
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.bin("cargo").exists()
            && self
                .rustup_home()
                .join("toolchains")
                .join(format!("{RUST_VERSION}-{}", host()))
                .exists()
    }

    /// Installs rustup if needed, then Rust [`RUST_VERSION`] with the minimal profile.
    /// Needs the network (D-060). `log` receives progress lines.
    ///
    /// # Errors
    /// A failed download or checksum, a failed rustup, or an I/O error.
    pub fn install(&self, log: &mut dyn FnMut(&str)) -> Result<(), Error> {
        if self.is_installed() {
            return Ok(());
        }
        if self.bin("rustup").exists() {
            log(&format!("Installing Rust {RUST_VERSION}…"));
            let mut rustup = self.command("rustup");
            rustup.args(["toolchain", "install", RUST_VERSION, "--profile", "minimal"]);
            return run(rustup, log);
        }

        let url = format!(
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init{}",
            host(),
            std::env::consts::EXE_SUFFIX
        );
        log(&format!("Downloading {url}…"));
        let installer = download(&url, 64 * 1024 * 1024)?;
        let checksum = download(&format!("{url}.sha256"), 1024)?;
        if !matches_checksum(&installer, &checksum) {
            return Err(Error::Checksum { url });
        }

        fs::create_dir_all(&self.data).map_err(|e| io_error(&self.data, e))?;
        let path = self
            .data
            .join(format!("rustup-init{}", std::env::consts::EXE_SUFFIX));
        fs::write(&path, &installer).map_err(|e| io_error(&path, e))?;
        make_executable(&path)?;
        log(&format!("Installing rustup and Rust {RUST_VERSION}…"));
        let mut init = Command::new(&path);
        init.args([
            "-y",
            "--no-modify-path",
            "--profile",
            "minimal",
            "--default-toolchain",
            RUST_VERSION,
        ])
        .env("RUSTUP_HOME", self.rustup_home())
        .env("CARGO_HOME", self.cargo_home())
        // A Rust from the system package manager is fine; it is not ours.
        .env("RUSTUP_INIT_SKIP_PATH_CHECK", "yes");
        let result = run(init, log);
        let _ = fs::remove_file(&path);
        result
    }

    /// Builds the project's `generated/` bot (written by `rustbot-codegen`) in debug, then
    /// copies the binary to `bots/<project id>/` in the data folder and returns its path.
    /// `log` receives cargo's output line by line. The first build needs the network to
    /// download crates (D-060).
    ///
    /// # Errors
    /// [`Error::NotInstalled`], a failed build, or an I/O error.
    pub fn build(&self, project: &Project, log: &mut dyn FnMut(&str)) -> Result<PathBuf, Error> {
        let built = self.cargo_build(project, false, log)?;
        let package = &project.manifest.bot.package;
        let dir = self.bot_dir(project);
        fs::create_dir_all(&dir).map_err(|e| io_error(&dir, e))?;
        let bot = dir.join(program_name(package));
        // Copy, then rename: a bot that is running keeps its old file.
        let tmp = dir.join(format!(".{package}.tmp"));
        fs::copy(&built, &tmp).map_err(|e| io_error(&built, e))?;
        fs::rename(&tmp, &bot).map_err(|e| io_error(&bot, e))?;
        Ok(bot)
    }

    /// Builds the project's `generated/` bot in release mode, for an export (D-135), and
    /// returns the binary's path in the shared target folder.
    ///
    /// # Errors
    /// As [`Toolchain::build`].
    pub fn build_release(
        &self,
        project: &Project,
        log: &mut dyn FnMut(&str),
    ) -> Result<PathBuf, Error> {
        self.cargo_build(project, true, log)
    }

    fn cargo_build(
        &self,
        project: &Project,
        release: bool,
        log: &mut dyn FnMut(&str),
    ) -> Result<PathBuf, Error> {
        if !self.is_installed() {
            return Err(Error::NotInstalled);
        }

        // Cargo reads `.cargo/config.toml` from its working folder and every folder above it,
        // so it runs in the app's cache folder: a project (or the repository around it)
        // cannot slip in a `rustc-wrapper`, a linker or another registry that way.
        fs::create_dir_all(&self.cache).map_err(|e| io_error(&self.cache, e))?;
        let manifest = project.root.join("generated").join("Cargo.toml");
        let mut cargo = self.command("cargo");
        cargo.args(["build", "--color", "never"]);
        if release {
            cargo.arg("--release");
        }
        cargo
            .arg("--manifest-path")
            .arg(&manifest)
            .current_dir(&self.cache)
            .env("CARGO_TARGET_DIR", self.target_dir());
        run(cargo, log)?;
        let profile = if release { "release" } else { "debug" };
        Ok(self
            .target_dir()
            .join(profile)
            .join(program_name(&project.manifest.bot.package)))
    }

    /// A rustup or cargo command of this toolchain. `RUSTUP_TOOLCHAIN` outranks every
    /// `rust-toolchain` file, so no folder can swap in another compiler.
    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(self.bin(program));
        command
            .env("RUSTUP_HOME", self.rustup_home())
            .env("CARGO_HOME", self.cargo_home())
            .env("RUSTUP_TOOLCHAIN", RUST_VERSION);
        command
    }
}

/// The file name of a bot's program: `<package>`, or `<package>.exe` on Windows.
#[must_use]
pub fn program_name(package: &str) -> String {
    format!("{package}{}", std::env::consts::EXE_SUFFIX)
}

/// The host's rustup target, e.g. `x86_64-unknown-linux-gnu`; MSVC on Windows (D-151).
fn host() -> String {
    let arch = std::env::consts::ARCH;
    if cfg!(windows) {
        format!("{arch}-pc-windows-msvc")
    } else {
        format!("{arch}-unknown-linux-gnu")
    }
}

/// Keeps a console program from opening a console window: the app is a window of its own
/// on Windows. Elsewhere it changes nothing.
pub fn hide_console(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Runs `command`, sending each line of its output to `log`.
fn run(mut command: Command, log: &mut dyn FnMut(&str)) -> Result<(), Error> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = hide_console(&mut command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io_error(Path::new(&program), e))?;
    if let Some(stderr) = child.stderr.take() {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            log(&line);
        }
    }
    let status = child.wait().map_err(|e| io_error(Path::new(&program), e))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Failed { program, status })
    }
}

fn download(url: &str, limit: u64) -> Result<Vec<u8>, Error> {
    let failed = |e: ureq::Error| Error::Download {
        url: url.to_owned(),
        message: e.to_string(),
    };
    ureq::get(url)
        .call()
        .map_err(failed)?
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(failed)
}

/// Whether `data` hashes to the SHA-256 in a `.sha256` file (`<hex>  *rustup-init`).
fn matches_checksum(data: &[u8], sha256_file: &[u8]) -> bool {
    let mut actual = String::new();
    for byte in Sha256::digest(data) {
        let _ = write!(actual, "{byte:02x}");
    }
    std::str::from_utf8(sha256_file)
        .ok()
        .and_then(|text| text.split_whitespace().next())
        .is_some_and(|expected| expected.eq_ignore_ascii_case(&actual))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|e| io_error(path, e))
}

/// Windows runs any `.exe`.
#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the same signature as on Unix
fn make_executable(_: &Path) -> Result<(), Error> {
    Ok(())
}

/// A C compiler on `PATH`, which every bot build needs (D-039).
#[cfg(not(windows))]
#[must_use]
pub fn c_compiler() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_program(&path, &["cc", "gcc"])
}

/// The folder of the Visual Studio Build Tools with the C++ tools, which bots for
/// `windows-msvc` need (D-151). rustc finds the linker in it by itself.
#[cfg(windows)]
#[must_use]
pub fn c_compiler() -> Option<PathBuf> {
    let program_files = std::env::var_os("ProgramFiles(x86)")?;
    let vswhere = PathBuf::from(program_files)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    let mut command = Command::new(vswhere);
    command.args([
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "installationPath",
    ]);
    let output = hide_console(&mut command).output().ok()?;
    let folder = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !folder.is_empty()).then(|| PathBuf::from(folder))
}

#[cfg(not(windows))]
fn find_program(path: &std::ffi::OsStr, names: &[&str]) -> Option<PathBuf> {
    std::env::split_paths(path)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// The command that installs the Build Tools' C++ tools through winget (D-151). Their
/// installer shows its own progress and Windows' administrator prompt. Run it only after
/// the user agreed, which includes the Build Tools' license.
#[cfg(windows)]
#[must_use]
pub fn install_c_compiler_command() -> Option<Command> {
    let mut command = Command::new("winget");
    command.args([
        "install",
        "--id",
        "Microsoft.VisualStudio.2022.BuildTools",
        "--exact",
        "--source",
        "winget",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--override",
        "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended",
    ]);
    hide_console(&mut command);
    Some(command)
}

/// The command that installs gcc through the system's administrator prompt (D-039), for the
/// package manager this system has. Run it only after the user agreed.
#[cfg(not(windows))]
#[must_use]
pub fn install_c_compiler_command() -> Option<Command> {
    let path = std::env::var_os("PATH")?;
    let (manager, args) = [
        ("dnf", &["install", "-y", "gcc"][..]),
        ("apt-get", &["install", "-y", "build-essential"][..]),
        (
            "pacman",
            &["-S", "--needed", "--noconfirm", "base-devel"][..],
        ),
        ("zypper", &["--non-interactive", "install", "gcc"][..]),
    ]
    .into_iter()
    .find_map(|(name, args)| find_program(&path, &[name]).map(|p| (p, args)))?;
    let mut command = Command::new("pkexec");
    command.arg(manager).args(args);
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::matches_checksum;

    #[test]
    fn checksums() {
        // SHA-256 of "abc".
        let file =
            b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad *rustup-init\n";
        assert!(matches_checksum(b"abc", file));
        assert!(!matches_checksum(b"abd", file));
        assert!(!matches_checksum(b"abc", b""));
    }

    #[cfg(not(windows))]
    #[test]
    fn programs_are_found_on_the_path() {
        use super::find_program;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gcc"), "").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(
            find_program(&path, &["cc", "gcc"]),
            Some(dir.path().join("gcc"))
        );
        assert_eq!(find_program(&path, &["clang"]), None);
    }
}
