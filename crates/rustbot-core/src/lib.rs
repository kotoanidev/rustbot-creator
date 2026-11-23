//! The project model of RustBot Creator: projects, graphs and the node registry.
//!
//! A bot project is a folder of text files (D-019, D-040, D-051):
//!
//! ```text
//! project.toml                     settings: the Manifest
//! graphs/commands/<name>.json      one Graph per command
//! graphs/commands/<name>.layout.json   its node positions (D-048)
//! graphs/events/<name>.json        one Graph per event handler
//! src/                             the user's own Rust modules (D-015)
//! generated/                       code generated from the graphs, committed (D-040)
//! ```
//!
//! Everything is written deterministically, so git diffs show only real changes.

mod core_nodes;
mod graph;
mod manifest;
mod project;
pub mod registry;
pub mod template;

pub use graph::{Edge, Graph, GraphKind, Layout, Node, NodeId, PinRef, PinRefError};
pub use manifest::{BotSettings, DjsRs, Gateway, Manifest, MemoryScope, MemoryType, MemoryVar};
pub use project::{Error, GraphFile, Project};
pub use registry::Registry;

/// The project format version this build reads and writes (D-051).
pub const SCHEMA: u32 = 1;

/// The djs-rs version new projects depend on.
pub const DJS_RS_VERSION: &str = "0.1.0-alpha.2";

/// The Rust version bots are built with, pinned per app release (D-062).
pub const RUST_VERSION: &str = "1.100.0";
