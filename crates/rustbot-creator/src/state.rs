//! What the panels show: the open project and what is selected in it. Every panel observes
//! one `Entity<AppState>` and redraws when it changes.

use std::collections::BTreeSet;
use std::time::Duration;

use gpui_kit::{Context, Task};
use rustbot_core::{GraphFile, NodeId, Project, Registry};

use crate::i18n::t_with;

/// How long autosave waits after the last change (D-068).
const SAVE_DELAY: Duration = Duration::from_millis(500);
/// How many changes can be undone.
const UNDO_LIMIT: usize = 200;
/// How many log lines are kept; older ones are dropped.
const LOG_LIMIT: usize = 5000;

pub(crate) struct AppState {
    pub(crate) project: Project,
    pub(crate) registry: Registry,
    /// The graph in the editor, as an index into `project.graphs`.
    pub(crate) open_graph: Option<usize>,
    /// The node whose form the properties panel shows.
    pub(crate) selected_node: Option<NodeId>,
    /// What the bot and its build printed, oldest first.
    pub(crate) logs: Vec<String>,
    /// The private toolchain, or `None` when the home folder is unknown.
    pub(crate) toolchain: Option<rustbot_toolchain::Toolchain>,
    /// What the bot is doing (D-071).
    pub(crate) run: crate::run::Run,
    /// Earlier versions of graphs, newest last: `(index in project.graphs, file)`.
    undo: Vec<(usize, GraphFile)>,
    redo: Vec<(usize, GraphFile)>,
    /// Graphs changed since they were last saved.
    unsaved: BTreeSet<usize>,
    save: Option<Task<()>>,
}

impl AppState {
    pub(crate) fn new(project: Project, cx: &mut Context<'_, Self>) -> Self {
        // Whatever is still waiting for autosave is written when the app quits.
        cx.on_app_quit(|this, _| {
            this.save_now();
            async {}
        })
        .detach();
        // The first command is open, so a new project starts on its /ping graph.
        let open_graph = (!project.graphs.is_empty()).then_some(0);
        Self {
            project,
            registry: Registry::with_core(),
            open_graph,
            selected_node: None,
            logs: Vec::new(),
            toolchain: rustbot_toolchain::Toolchain::locate().ok(),
            run: crate::run::Run::Stopped,
            undo: Vec::new(),
            redo: Vec::new(),
            unsaved: BTreeSet::new(),
            save: None,
        }
    }

    /// Adds a line to the Logs panel.
    pub(crate) fn log(&mut self, line: String) {
        if self.logs.len() >= LOG_LIMIT {
            self.logs.drain(..LOG_LIMIT / 10);
        }
        self.logs.push(line);
    }

    /// The graph in the editor.
    pub(crate) fn open_file(&self) -> Option<&GraphFile> {
        self.project.graphs.get(self.open_graph?)
    }

    /// Changes the open graph with `change`, which `undoable` lets Ctrl+Z take back. A drag
    /// is one undoable step followed by plain ones. It is saved after a short pause.
    pub(crate) fn edit(
        &mut self,
        undoable: bool,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut GraphFile, &Registry),
    ) {
        let Some(ix) = self.open_graph else { return };
        let Some(file) = self.project.graphs.get_mut(ix) else {
            return;
        };
        if undoable {
            self.undo.push((ix, file.clone()));
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        change(file, &self.registry);
        self.changed(ix, cx);
    }

    /// Takes back the last undoable change, or redoes the last undone one.
    pub(crate) fn undo(&mut self, redo: bool, cx: &mut Context<'_, Self>) {
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let Some((ix, file)) = from.pop() else { return };
        let Some(current) = self.project.graphs.get_mut(ix) else {
            return;
        };
        to.push((ix, std::mem::replace(current, file)));
        self.open_graph = Some(ix);
        let selected_gone = self
            .selected_node
            .as_ref()
            .is_some_and(|node| !self.project.graphs[ix].graph.nodes.contains_key(node));
        if selected_gone {
            self.selected_node = None;
        }
        self.changed(ix, cx);
    }

    fn changed(&mut self, ix: usize, cx: &mut Context<'_, Self>) {
        self.unsaved.insert(ix);
        self.save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            let _ = this.update(cx, |this, cx| {
                this.save_now();
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Writes every graph changed since the last save. A failure goes to the logs and the
    /// graph stays unsaved, so the next change tries again.
    pub(crate) fn save_now(&mut self) {
        for ix in std::mem::take(&mut self.unsaved) {
            let Some(file) = self.project.graphs.get(ix) else {
                continue;
            };
            if let Err(e) = self.project.save_graph(file) {
                let error = e.to_string();
                self.logs.push(t_with(
                    "error-save",
                    &[("file", &file.name), ("error", &error)],
                ));
                self.unsaved.insert(ix);
            }
        }
    }
}
