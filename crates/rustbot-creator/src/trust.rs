//! Project trust (D-064). Projects made in the app are trusted; a project from elsewhere
//! asks once before its first build. Trusted folders are listed one canonical path per
//! line in `trusted-projects` in the app's data folder, so a copy somewhere else asks
//! again.

use std::io;
use std::path::{Path, PathBuf};

fn list(data: &Path) -> PathBuf {
    data.join("trusted-projects")
}

/// The folder as it is listed, or `None` for a path that cannot be one line.
fn key(root: &Path) -> Option<String> {
    let path = root.canonicalize().ok()?;
    let text = path.to_str()?;
    (!text.contains('\n')).then(|| text.to_owned())
}

pub(crate) fn is_trusted(data: &Path, root: &Path) -> bool {
    let Some(key) = key(root) else { return false };
    std::fs::read_to_string(list(data)).is_ok_and(|text| text.lines().any(|line| line == key))
}

/// Remembers the folder as trusted. A path that cannot be listed stays untrusted and asks
/// every time.
pub(crate) fn trust(data: &Path, root: &Path) -> io::Result<()> {
    let Some(key) = key(root) else { return Ok(()) };
    if is_trusted(data, root) {
        return Ok(());
    }
    std::fs::create_dir_all(data)?;
    let mut text = std::fs::read_to_string(list(data)).unwrap_or_default();
    text.push_str(&key);
    text.push('\n');
    std::fs::write(list(data), text)
}

#[cfg(test)]
mod tests {
    #[test]
    fn trust_is_per_folder() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let (one, other) = (dir.path().join("one"), dir.path().join("other"));
        std::fs::create_dir_all(&one).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        assert!(!super::is_trusted(&data, &one));
        super::trust(&data, &one).unwrap();
        super::trust(&data, &one).unwrap();
        assert!(super::is_trusted(&data, &one));
        // The same folder by another spelling is the same folder.
        assert!(super::is_trusted(&data, &one.join("../one")));
        assert!(!super::is_trusted(&data, &other));
        let text = std::fs::read_to_string(data.join("trusted-projects")).unwrap();
        assert_eq!(text.lines().count(), 1);
    }
}
