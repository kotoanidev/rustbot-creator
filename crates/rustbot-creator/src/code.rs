//! The user's own Rust code in the project's `src/` (D-015, D-175, D-176).

use std::fs;
use std::io;
use std::path::Path;

/// The first lines of a new `src/lib.rs`.
const HEADER: &str = "//! The bot's own Rust code (D-015). \"Custom code\" nodes call the functions \
                      here, and\n//! \"Create function\" adds them. RustBot Creator never \
                      changes what you write.\n";

/// Adds `stub`, the function `function`, to the project's `src/lib.rs`, and makes the file
/// when there is none. `false` when it has a function of that name already. A link is
/// refused, since a project from elsewhere could point it at another file.
pub(crate) fn add_function(root: &Path, function: &str, stub: &str) -> io::Result<bool> {
    let src = root.join("src");
    match fs::symlink_metadata(&src) {
        Ok(metadata) if !metadata.is_dir() => {
            return Err(io::Error::other("src is not a folder"));
        }
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => fs::create_dir(&src)?,
        Err(e) => return Err(e),
    }
    let lib = src.join("lib.rs");
    let mut text = match fs::symlink_metadata(&lib) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(io::Error::other("src/lib.rs is not a file"));
        }
        Ok(_) => fs::read_to_string(&lib)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => HEADER.to_owned(),
        Err(e) => return Err(e),
    };
    if has_function(&text, function) {
        return Ok(false);
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(stub);
    fs::write(&lib, text)?;
    Ok(true)
}

/// The 0-based line of `fn <name>` in `code`, to show a new function.
pub(crate) fn function_line(code: &str, name: &str) -> Option<u32> {
    let line = code.lines().position(|line| has_function(line, name))?;
    u32::try_from(line).ok()
}

/// `relative` inside the project when it is a path under `src/` of plain names, through
/// no link, so a project from elsewhere cannot point the editor at another file.
fn checked(root: &Path, relative: &Path) -> io::Result<std::path::PathBuf> {
    let refused = || io::Error::other(format!("{} is not a file in src/", relative.display()));
    let mut parts = relative.components();
    if parts.next() != Some(std::path::Component::Normal("src".as_ref())) {
        return Err(refused());
    }
    let mut path = root.join("src");
    for part in std::iter::once(None).chain(parts.map(Some)) {
        if let Some(part) = part {
            let std::path::Component::Normal(part) = part else {
                return Err(refused());
            };
            path.push(part);
        }
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(refused()),
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(path)
}

/// A file of the user's code, `src/…`.
pub(crate) fn read(root: &Path, relative: &Path) -> io::Result<String> {
    fs::read_to_string(checked(root, relative)?)
}

/// Writes a file of the user's code, `src/…`, whose folder exists.
pub(crate) fn write(root: &Path, relative: &Path, text: &str) -> io::Result<()> {
    fs::write(checked(root, relative)?, text)
}

/// Whether `name` can be a new file of the user's code: a module name with `.rs`.
pub(crate) fn valid_file_name(name: &str) -> bool {
    name.strip_suffix(".rs")
        .is_some_and(|stem| rustbot_core::registry::is_rust_name(stem) && stem != "lib")
}

/// Makes `src/<name>`, a new module file, with a note on how to use it. The path, or
/// `AlreadyExists`.
pub(crate) fn create_file(root: &Path, name: &str) -> io::Result<std::path::PathBuf> {
    let relative = Path::new("src").join(name);
    let path = checked(root, &relative)?;
    let src = root.join("src");
    if fs::symlink_metadata(&src).is_err() {
        fs::create_dir(&src)?;
    }
    let module = name.trim_end_matches(".rs");
    let note = format!("//! Add `mod {module};` to src/lib.rs to use this file.\n");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| io::Write::write_all(&mut file, note.as_bytes()))?;
    Ok(relative)
}

/// Whether `code` has `fn <name>(` or `fn <name><`, however it is spaced.
fn has_function(code: &str, name: &str) -> bool {
    code.match_indices("fn ").any(|(at, _)| {
        code[at + 3..]
            .trim_start()
            .strip_prefix(name)
            .is_some_and(|after| after.trim_start().starts_with(['(', '<']))
    })
}

#[cfg(test)]
mod tests {
    use super::add_function;

    #[test]
    fn files_are_read_and_written_only_in_src_without_links() {
        use std::path::Path;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(super::valid_file_name("points.rs"));
        for bad in [
            "lib.rs",
            "Points.rs",
            "points",
            "a/b.rs",
            "../x.rs",
            "fn.rs",
        ] {
            assert!(!super::valid_file_name(bad), "{bad}");
        }
        let made = super::create_file(root, "points.rs").unwrap();
        assert_eq!(made, Path::new("src/points.rs"));
        assert!(super::read(root, &made).unwrap().contains("mod points;"));
        assert!(super::create_file(root, "points.rs").is_err());
        super::write(root, &made, "pub fn x() {}\n").unwrap();
        assert_eq!(super::read(root, &made).unwrap(), "pub fn x() {}\n");
        for outside in [
            "generated/src/main.rs",
            "src/../project.toml",
            "/etc/hostname",
        ] {
            assert!(super::read(root, Path::new(outside)).is_err(), "{outside}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/hostname", root.join("src/host.rs")).unwrap();
            assert!(super::read(root, Path::new("src/host.rs")).is_err());
            assert!(super::write(root, Path::new("src/host.rs"), "x").is_err());
        }
        let code = "//! x\n\npub async fn give(\n";
        assert_eq!(super::function_line(code, "give"), Some(2));
    }

    #[test]
    fn a_function_is_added_once_and_links_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let stub = "pub async fn give() {}\n";
        assert!(add_function(dir.path(), "give", stub).unwrap());
        let lib = dir.path().join("src/lib.rs");
        let text = std::fs::read_to_string(&lib).unwrap();
        assert!(text.starts_with("//! The bot's own Rust code"));
        assert!(text.ends_with("\n\npub async fn give() {}\n"));
        assert!(!add_function(dir.path(), "give", stub).unwrap());
        assert!(add_function(dir.path(), "giv", "pub async fn giv() {}\n").unwrap());
        assert_eq!(
            std::fs::read_to_string(&lib)
                .unwrap()
                .matches("fn giv")
                .count(),
            2
        );

        #[cfg(unix)]
        {
            let other = tempfile::tempdir().unwrap();
            std::fs::create_dir(other.path().join("src")).unwrap();
            std::os::unix::fs::symlink("/etc/hostname", other.path().join("src/lib.rs")).unwrap();
            assert!(add_function(other.path(), "give", stub).is_err());
            let linked = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(dir.path().join("src"), linked.path().join("src")).unwrap();
            assert!(add_function(linked.path(), "other", stub).is_err());
        }
    }
}
