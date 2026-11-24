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
