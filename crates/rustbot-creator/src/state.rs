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

/// Where a line in the Logs panel comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// The app itself: steps, results and errors.
    App,
    /// The output of installing the toolchain or building the bot.
    Build,
    /// What the running bot prints.
    Bot,
}

/// How serious a line is, which sets its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Info,
    Warn,
    Error,
}

/// A line in the Logs panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Line {
    pub(crate) source: Source,
    pub(crate) level: Level,
    pub(crate) text: String,
}

impl Line {
    /// A line of the build's or the bot's output. Its level is read from the text: cargo's
    /// `error:`/`warning:` and tracing's `ERROR`/`WARN`. Anything else the bot writes to
    /// stderr is an error: a panic, or the error it exits with.
    pub(crate) fn output(source: Source, text: String, stderr: bool) -> Self {
        let level = level_of(&text).unwrap_or(if stderr && source == Source::Bot {
            Level::Error
        } else {
            Level::Info
        });
        Self {
            source,
            level,
            text,
        }
    }
}

fn level_of(text: &str) -> Option<Level> {
    let start = text.trim_start();
    if start.starts_with("error:") || start.starts_with("error[") {
        return Some(Level::Error);
    }
    if start.starts_with("warning:") {
        return Some(Level::Warn);
    }
    // tracing: "2026-11-23T10:00:00.000000Z  WARN target: message"
    text.split_whitespace().take(2).find_map(|word| match word {
        "ERROR" => Some(Level::Error),
        "WARN" => Some(Level::Warn),
        "INFO" | "DEBUG" | "TRACE" => Some(Level::Info),
        _ => None,
    })
}

pub(crate) struct AppState {
    pub(crate) project: Project,
    pub(crate) registry: Registry,
    /// The graph in the editor, as an index into `project.graphs`.
    pub(crate) open_graph: Option<usize>,
    /// The node whose form the properties panel shows.
    pub(crate) selected_node: Option<NodeId>,
    /// Requests for the editor to bring the selected node into view, counted (D-090).
    pub(crate) reveal: u64,
    /// What the app, the build and the bot printed, oldest first.
    pub(crate) logs: Vec<Line>,
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
            reveal: 0,
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
    pub(crate) fn push(&mut self, line: Line) {
        if self.logs.len() >= LOG_LIMIT {
            self.logs.drain(..LOG_LIMIT / 10);
        }
        self.logs.push(line);
    }

    /// Adds a message of the app to the Logs panel.
    pub(crate) fn log(&mut self, text: String) {
        self.push(Line {
            source: Source::App,
            level: Level::Info,
            text,
        });
    }

    /// Adds an error of the app to the Logs panel.
    pub(crate) fn log_error(&mut self, text: String) {
        self.push(Line {
            source: Source::App,
            level: Level::Error,
            text,
        });
    }

    /// Opens graph `graph` with `node` selected, and has the editor show it (D-090).
    pub(crate) fn show_node(
        &mut self,
        graph: usize,
        node: Option<NodeId>,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_graph = Some(graph);
        self.selected_node = node;
        self.reveal += 1;
        cx.notify();
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
                let text = t_with("error-save", &[("file", &file.name), ("error", &error)]);
                self.log_error(text);
                self.unsaved.insert(ix);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Level, Line, Source};

    fn level(source: Source, text: &str, stderr: bool) -> Level {
        Line::output(source, text.to_owned(), stderr).level
    }

    #[test]
    fn levels_come_from_cargo_tracing_and_stderr() {
        let build = |text| level(Source::Build, text, true);
        assert_eq!(build("error[E0425]: cannot find value `x`"), Level::Error);
        assert_eq!(build("warning: unused variable: `y`"), Level::Warn);
        assert_eq!(build("   Compiling djs-rs v0.1.0-alpha.2"), Level::Info);
        let bot = |text, stderr| level(Source::Bot, text, stderr);
        assert_eq!(
            bot("2026-11-23T10:00:00.1Z  WARN djs_rs: slow", false),
            Level::Warn
        );
        assert_eq!(
            bot("2026-11-23T10:00:00.1Z ERROR djs_rs: failed", false),
            Level::Error
        );
        assert_eq!(
            bot("2026-11-23T10:00:00.1Z  INFO djs_rs: ready", true),
            Level::Info
        );
        assert_eq!(bot("Ready as bot#0001", false), Level::Info);
        assert_eq!(bot("Error: Rest(...)", true), Level::Error);
        assert_eq!(bot("errors are fine here", false), Level::Info);
    }
}
