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
    /// The project has cargo or rustup settings that would run before or instead of the
    /// build, e.g. a `.cargo/config.toml` with a `rustc-wrapper`. Cargo reads them from the
    /// project folder too, so a project from elsewhere could hide code in them.
    #[error("{}: build settings in a project are not allowed", .0.display())]
    ProjectConfig(PathBuf),
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
        self.cargo_home().join("bin").join(program)
    }

    /// The shared target folder of every bot (D-063).
    #[must_use]
    pub fn target_dir(&self) -> PathBuf {
        self.cache.join("target")
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
            "https://static.rust-lang.org/rustup/dist/{}/rustup-init",
            host()
        );
        log(&format!("Downloading {url}…"));
        let installer = download(&url, 64 * 1024 * 1024)?;
        let checksum = download(&format!("{url}.sha256"), 1024)?;
        if !matches_checksum(&installer, &checksum) {
            return Err(Error::Checksum { url });
        }

        fs::create_dir_all(&self.data).map_err(|e| io_error(&self.data, e))?;
        let path = self.data.join("rustup-init");
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
    /// [`Error::ProjectConfig`], [`Error::NotInstalled`], a failed build, or an I/O error.
    pub fn build(&self, project: &Project, log: &mut dyn FnMut(&str)) -> Result<PathBuf, Error> {
        // `generated/` is cleaned by codegen; the project root is checked here.
        for name in [".cargo", "rust-toolchain", "rust-toolchain.toml"] {
            let path = project.root.join(name);
            if fs::symlink_metadata(&path).is_ok() {
                return Err(Error::ProjectConfig(path));
            }
        }
        if !self.is_installed() {
            return Err(Error::NotInstalled);
        }

        let generated = project.root.join("generated");
        let mut cargo = self.command("cargo");
        cargo
            .args(["build", "--color", "never"])
            .current_dir(&generated)
            .env("CARGO_TARGET_DIR", self.target_dir());
        run(cargo, log)?;

        let package = &project.manifest.bot.package;
        let built = self.target_dir().join("debug").join(package);
        let dir = self.data.join("bots").join(project.manifest.id.to_string());
        fs::create_dir_all(&dir).map_err(|e| io_error(&dir, e))?;
        let bot = dir.join(package);
        // Copy, then rename: a bot that is running keeps its old file.
        let tmp = dir.join(format!(".{package}.tmp"));
        fs::copy(&built, &tmp).map_err(|e| io_error(&built, e))?;
        fs::rename(&tmp, &bot).map_err(|e| io_error(&bot, e))?;
        Ok(bot)
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

/// The host's rustup target, e.g. `x86_64-unknown-linux-gnu`.
fn host() -> String {
    format!("{}-unknown-linux-gnu", std::env::consts::ARCH)
}

/// Runs `command`, sending each line of its output to `log`.
fn run(mut command: Command, log: &mut dyn FnMut(&str)) -> Result<(), Error> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
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

fn make_executable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|e| io_error(path, e))
}

/// A C compiler on `PATH`, which every bot build needs (D-039).
#[must_use]
pub fn c_compiler() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_program(&path, &["cc", "gcc"])
}

fn find_program(path: &std::ffi::OsStr, names: &[&str]) -> Option<PathBuf> {
    std::env::split_paths(path)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

/// The command that installs gcc through the system's administrator prompt (D-039), for the
/// package manager this system has. Run it only after the user agreed.
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
    use super::{find_program, matches_checksum};

    #[test]
    fn checksums() {
        // SHA-256 of "abc".
        let file =
            b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad *rustup-init\n";
        assert!(matches_checksum(b"abc", file));
        assert!(!matches_checksum(b"abd", file));
        assert!(!matches_checksum(b"abc", b""));
    }

    #[test]
    fn programs_are_found_on_the_path() {
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
