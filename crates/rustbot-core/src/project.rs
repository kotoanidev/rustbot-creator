use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{Graph, GraphKind, Layout, Manifest, SCHEMA};

const MANIFEST: &str = "project.toml";
const LAYOUT_SUFFIX: &str = ".layout.json";
const GITIGNORE: &str = "target/\n.env\n";

/// An error loading or saving a project.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read or written.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// `project.toml` is not valid.
    #[error("{}: {source}", .path.display())]
    Toml {
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: toml::de::Error,
    },
    /// A graph or layout file is not valid.
    #[error("{}: {source}", .path.display())]
    Json {
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: serde_json::Error,
    },
    /// The file was written by a different version of the project format.
    #[error(
        "{}: project format version {found} is not supported (this version reads {SCHEMA})",
        .path.display()
    )]
    UnsupportedSchema {
        /// The file.
        path: PathBuf,
        /// The version found in it.
        found: u32,
    },
    /// [`Project::create`] found a project already in the folder.
    #[error("{}: a project already exists here", .0.display())]
    AlreadyExists(PathBuf),
    /// A graph name that cannot be a file name.
    #[error("{0:?} is not a valid graph name")]
    InvalidGraphName(String),
    /// A file or folder of the project is a symbolic link. Projects can come from untrusted
    /// repositories, and a link could point reads or writes outside the project.
    #[error("{}: symbolic links are not allowed in a project", .0.display())]
    Symlink(PathBuf),
    /// A path that is absolute or leaves the project folder.
    #[error("{}: not a path inside the project", .0.display())]
    InvalidPath(PathBuf),
}

/// A graph file with its layout, as loaded from `graphs/<kind>/<name>.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphFile {
    /// The folder it lives in.
    pub kind: GraphKind,
    /// The file name without `.json`.
    pub name: String,
    /// The logic.
    pub graph: Graph,
    /// The node positions. Empty when the layout file is missing.
    pub layout: Layout,
}

/// A bot project on disk (D-019, D-040, D-051).
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// The project folder.
    pub root: PathBuf,
    /// `project.toml`.
    pub manifest: Manifest,
    /// Every graph, sorted by kind and name.
    pub graphs: Vec<GraphFile>,
}

impl Project {
    /// Creates a new project in `root`, writing `project.toml` and `.gitignore`.
    ///
    /// # Errors
    /// [`Error::AlreadyExists`] if `root` already holds a project, or an I/O error.
    pub fn create(root: impl Into<PathBuf>, manifest: Manifest) -> Result<Self, Error> {
        let root = root.into();
        let manifest_path = root.join(MANIFEST);
        if manifest_path.exists() {
            return Err(Error::AlreadyExists(root));
        }
        fs::create_dir_all(&root).map_err(|source| io_error(&root, source))?;
        let project = Self {
            root,
            manifest,
            graphs: Vec::new(),
        };
        project.save_manifest()?;
        let gitignore = project.root.join(".gitignore");
        if !gitignore.exists() {
            write_atomic(&gitignore, GITIGNORE)?;
        }
        Ok(project)
    }

    /// Loads the project in `root`, with all of its graphs.
    ///
    /// # Errors
    /// An I/O error, an invalid file, or a file in an unsupported format version.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, Error> {
        let root = root.into();
        let manifest_path = root.join(MANIFEST);
        let text = read(&manifest_path)?;
        let manifest: Manifest = toml::from_str(&text).map_err(|source| Error::Toml {
            path: manifest_path.clone(),
            source,
        })?;
        check_schema(&manifest_path, manifest.schema)?;

        let mut graphs = Vec::new();
        reject_symlink(&root.join("graphs"))?;
        for kind in GraphKind::ALL {
            let dir = root.join("graphs").join(kind.dir());
            reject_symlink(&dir)?;
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(source) => return Err(io_error(&dir, source)),
            };
            for entry in entries {
                let path = entry.map_err(|source| io_error(&dir, source))?.path();
                let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if file_name.ends_with(LAYOUT_SUFFIX) {
                    continue;
                }
                let Some(name) = file_name.strip_suffix(".json") else {
                    continue;
                };
                let graph: Graph = read_json(&path)?;
                check_schema(&path, graph.schema)?;
                let layout_path = dir.join(format!("{name}{LAYOUT_SUFFIX}"));
                let layout = if layout_path.exists() {
                    let layout: Layout = read_json(&layout_path)?;
                    check_schema(&layout_path, layout.schema)?;
                    layout
                } else {
                    Layout::default()
                };
                graphs.push(GraphFile {
                    kind,
                    name: name.to_owned(),
                    graph,
                    layout,
                });
            }
        }
        graphs.sort_by(|a, b| (a.kind, &a.name).cmp(&(b.kind, &b.name)));
        Ok(Self {
            root,
            manifest,
            graphs,
        })
    }

    /// Writes `project.toml`.
    ///
    /// # Errors
    /// An I/O error.
    pub fn save_manifest(&self) -> Result<(), Error> {
        write_atomic(&self.root.join(MANIFEST), &to_toml(&self.manifest))
    }

    /// Writes a graph and its layout to `graphs/<kind>/<name>.json` and
    /// `<name>.layout.json`.
    ///
    /// # Errors
    /// [`Error::InvalidGraphName`] for a name that cannot be a file name, or an I/O error.
    pub fn save_graph(&self, file: &GraphFile) -> Result<(), Error> {
        let name = &file.name;
        let valid = !name.is_empty()
            && !name.starts_with('.')
            && !name.contains(['/', '\\'])
            && !name.ends_with(".layout");
        if !valid {
            return Err(Error::InvalidGraphName(name.clone()));
        }
        let graphs = self.root.join("graphs");
        let dir = graphs.join(file.kind.dir());
        reject_symlink(&graphs)?;
        reject_symlink(&dir)?;
        fs::create_dir_all(&dir).map_err(|source| io_error(&dir, source))?;
        write_atomic(&dir.join(format!("{name}.json")), &to_json(&file.graph))?;
        write_atomic(
            &dir.join(format!("{name}{LAYOUT_SUFFIX}")),
            &to_json(&file.layout),
        )
    }
}

/// Other files of the project, such as `generated/`. Paths are relative to the project folder,
/// and symbolic links are refused on the way, as for every other project file.
impl Project {
    /// Writes `contents` to `relative`, creating its folders.
    ///
    /// # Errors
    /// [`Error::InvalidPath`], [`Error::Symlink`] or an I/O error.
    pub fn write_file(&self, relative: &Path, contents: &str) -> Result<(), Error> {
        let path = self.checked(relative)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
        }
        write_atomic(&path, contents)
    }

    /// Removes the file `relative`. A missing file is not an error.
    ///
    /// # Errors
    /// [`Error::InvalidPath`], [`Error::Symlink`] or an I/O error.
    pub fn remove_file(&self, relative: &Path) -> Result<(), Error> {
        let path = self.checked(relative)?;
        match fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_error(&path, e)),
            _ => Ok(()),
        }
    }

    /// Every file under the folder `relative`, recursively and sorted, as paths relative to
    /// the project folder. A missing folder has none.
    ///
    /// # Errors
    /// [`Error::InvalidPath`], [`Error::Symlink`] or an I/O error.
    pub fn files_under(&self, relative: &Path) -> Result<Vec<PathBuf>, Error> {
        let mut files = Vec::new();
        let mut dirs = vec![self.checked(relative)?];
        while let Some(dir) = dirs.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(source) => return Err(io_error(&dir, source)),
            };
            for entry in entries {
                let path = entry.map_err(|source| io_error(&dir, source))?.path();
                reject_symlink(&path)?;
                if path.is_dir() {
                    dirs.push(path);
                } else if let Ok(relative) = path.strip_prefix(&self.root) {
                    files.push(relative.to_owned());
                }
            }
        }
        files.sort();
        Ok(files)
    }

    fn checked(&self, relative: &Path) -> Result<PathBuf, Error> {
        let mut path = self.root.clone();
        for component in relative.components() {
            let std::path::Component::Normal(part) = component else {
                return Err(Error::InvalidPath(relative.to_owned()));
            };
            path.push(part);
            reject_symlink(&path)?;
        }
        Ok(path)
    }
}

fn check_schema(path: &Path, found: u32) -> Result<(), Error> {
    if found == SCHEMA {
        Ok(())
    } else {
        Err(Error::UnsupportedSchema {
            path: path.to_owned(),
            found,
        })
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let text = read(path)?;
    serde_json::from_str(&text).map_err(|source| Error::Json {
        path: path.to_owned(),
        source,
    })
}

fn to_toml(manifest: &Manifest) -> String {
    toml::to_string_pretty(manifest).expect("the manifest always serializes to TOML")
}

/// Pretty JSON with a trailing newline, the same bytes for the same value.
fn to_json(value: &impl Serialize) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("graphs always serialize to JSON");
    text.push('\n');
    text
}

fn read(path: &Path) -> Result<String, Error> {
    reject_symlink(path)?;
    fs::read_to_string(path).map_err(|source| io_error(path, source))
}

fn reject_symlink(path: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(Error::Symlink(path.to_owned())),
        _ => Ok(()),
    }
}

/// Writes through a temporary file and a rename, so a crash never leaves a half-written file.
///
/// The temporary file is created with `create_new`, which never follows an existing path,
/// even a symlink planted under that name. If one is there, only the link is removed. The
/// rename replaces `path` itself, never what a link at `path` points to.
fn write_atomic(path: &Path, contents: &str) -> Result<(), Error> {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    let create = || {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
    };
    let mut file = match create() {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_file(&tmp).map_err(|source| io_error(&tmp, source))?;
            create()
        }
        other => other,
    }
    .map_err(|source| io_error(&tmp, source))?;
    file.write_all(contents.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| io_error(&tmp, source))?;
    drop(file);
    fs::rename(&tmp, path).map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}
