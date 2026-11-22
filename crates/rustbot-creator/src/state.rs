//! What the panels show: the open project and what is selected in it. Every panel observes
//! one `Entity<AppState>` and redraws when it changes.

use rustbot_core::{NodeId, Project, Registry};

pub(crate) struct AppState {
    pub(crate) project: Project,
    pub(crate) registry: Registry,
    /// The graph in the editor, as an index into `project.graphs`.
    pub(crate) open_graph: Option<usize>,
    /// The node whose form the properties panel shows.
    pub(crate) selected_node: Option<NodeId>,
    /// What the bot and its build printed, oldest first.
    pub(crate) logs: Vec<String>,
}

impl AppState {
    pub(crate) fn new(project: Project) -> Self {
        // The first command is open, so a new project starts on its /ping graph.
        let open_graph = (!project.graphs.is_empty()).then_some(0);
        Self {
            project,
            registry: Registry::with_core(),
            open_graph,
            selected_node: None,
            logs: Vec::new(),
        }
    }
}
