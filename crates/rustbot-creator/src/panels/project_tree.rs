//! The project tree in the side bar: the project's commands, events and memory.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _,
    PromptLevel, Render, Styled as _, Subscription, WeakEntity, Window, div, px,
};
use rustbot_core::registry::{Category, valid_command_name, valid_memory_name};
use rustbot_core::{GraphKind, NodeId, Project};

use crate::i18n::{t, t_with};
use crate::panels::editor::node_name;
use crate::state::AppState;

pub(crate) struct ProjectTree {
    focus: FocusHandle,
    state: Entity<AppState>,
    tree: Entity<TreeState>,
    /// The name field of a command being added (D-092) or a memory variable being renamed
    /// (D-109).
    adding: Option<(Adding, Entity<InputState>, Subscription)>,
    error: Option<String>,
}

super::dock_panel!(ProjectTree, "ProjectTree", "panel-project");

/// What a name typed in the tree is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Adding {
    Command,
    /// A new name for memory variable `ix`.
    Rename(usize),
}

/// The tree's folder of memory variables (D-109). A variable's id is `memory:<index>`, and
/// a node using it is `use:<index>:<graph>:<node>`.
const MEMORY: &str = "memory";

impl ProjectTree {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        let initial = items(&state.read(cx).project);
        let tree = cx.new(|cx| TreeState::new(cx).items(initial));
        cx.observe(&state, |this, state, cx| {
            let items = items(&state.read(cx).project);
            this.tree.update(cx, |tree, cx| tree.set_items(items, cx));
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            state,
            tree,
            adding: None,
            error: None,
        }
    }

    /// Shows the name field for a new command or a memory variable's new name.
    fn start_adding(&mut self, what: Adding, window: &mut Window, cx: &mut Context<'_, Self>) {
        let (placeholder, name) = match what {
            Adding::Command => (t("new-command-placeholder"), String::new()),
            Adding::Rename(ix) => (
                t("new-memory-placeholder"),
                self.state
                    .read(cx)
                    .project
                    .manifest
                    .memory
                    .get(ix)
                    .map_or_else(String::new, |var| var.name.clone()),
            ),
        };
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(name)
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        let changes = cx.subscribe_in(
            &input,
            window,
            move |this, input, event, _, cx| match event {
                InputEvent::PressEnter { .. } => {
                    let name = input.read(cx).value().trim().to_owned();
                    match what {
                        Adding::Command => this.add_command(&name, cx),
                        Adding::Rename(ix) => this.rename_memory(ix, &name, cx),
                    }
                }
                // Leaving an empty field gives up.
                InputEvent::Blur if input.read(cx).value().trim().is_empty() => {
                    this.adding = None;
                    this.error = None;
                    cx.notify();
                }
                _ => {}
            },
        );
        self.adding = Some((what, input, changes));
        self.error = None;
        cx.notify();
    }

    fn add_command(&mut self, name: &str, cx: &mut Context<'_, Self>) {
        let state = self.state.read(cx);
        self.error = if !valid_command_name(name) {
            Some(t("new-command-invalid"))
        } else if state.free_name(GraphKind::Command, name) != name {
            Some(t("new-command-taken"))
        } else {
            self.state
                .update(cx, |state, cx| {
                    state.add_graph(GraphKind::Command, name, "core.slash_command", cx)
                })
                .err()
                .map(|e| e.to_string())
        };
        if self.error.is_none() {
            self.adding = None;
        }
        cx.notify();
    }
}

impl ProjectTree {
    fn rename_memory(&mut self, ix: usize, name: &str, cx: &mut Context<'_, Self>) {
        let memory = &self.state.read(cx).project.manifest.memory;
        let unchanged = memory.get(ix).is_some_and(|var| var.name == name);
        let taken = memory.iter().any(|var| var.name == name);
        self.error = if unchanged {
            None
        } else if !valid_memory_name(name) {
            Some(t("new-memory-invalid"))
        } else if taken {
            Some(t("new-memory-taken"))
        } else {
            self.state
                .update(cx, |state, cx| state.rename_memory(ix, name, cx));
            None
        };
        if self.error.is_none() {
            self.adding = None;
        }
        cx.notify();
    }
}

/// Adds a graph for the event `trigger`, named after it (D-092).
fn add_event(state: &Entity<AppState>, trigger: &str, cx: &mut App) {
    state.update(cx, |state, cx| {
        let name = state.free_name(GraphKind::Event, trigger.trim_start_matches("core."));
        if let Err(e) = state.add_graph(GraphKind::Event, &name, trigger, cx) {
            state.log_error(e.to_string());
        }
    });
}

/// Asks before deleting `name`, which cannot be undone (D-092): a graph, whose file goes,
/// or a memory variable (`detail` says which), then runs `delete`.
fn confirm_delete(
    state: &Entity<AppState>,
    name: &str,
    detail: &str,
    delete: impl FnOnce(&mut AppState, &mut Context<'_, AppState>) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    let (yes, cancel) = (t("delete"), t("cancel"));
    let answer = window.prompt(
        PromptLevel::Warning,
        &t_with("delete-graph-title", &[("name", name)]),
        Some(&t(detail)),
        &[yes.as_str(), cancel.as_str()],
        cx,
    );
    let state = state.clone();
    window
        .spawn(cx, async move |cx| {
            if answer.await == Ok(0) {
                let _ = cx.update(|_, cx| state.update(cx, delete));
            }
        })
        .detach();
}

/// A folder per graph kind, where a graph's id is its index in `project.graphs`, and the
/// memory variables with the nodes that use them (D-109).
fn items(project: &Project) -> Vec<TreeItem> {
    let mut items: Vec<TreeItem> = [
        (GraphKind::Command, "tree-commands"),
        (GraphKind::Event, "tree-events"),
    ]
    .into_iter()
    .map(|(kind, label)| {
        let graphs = project.graphs.iter().enumerate();
        TreeItem::new(kind.dir(), t(label)).expanded(true).children(
            graphs
                .filter(|(_, file)| file.kind == kind)
                .map(|(ix, file)| TreeItem::new(ix.to_string(), file.name.clone())),
        )
    })
    .collect();
    let memory = project.manifest.memory.iter().enumerate().map(|(ix, var)| {
        let uses = crate::state::memory_uses(project, &var.name);
        let uses = uses.into_iter().map(|(graph, node)| {
            let file = &project.graphs[graph];
            let place = match file.kind {
                GraphKind::Command => format!("/{}", file.name),
                GraphKind::Event => file.name.clone(),
            };
            let label = format!("{place} › {}", node_name(&file.graph.nodes[&node].kind));
            TreeItem::new(format!("use:{ix}:{graph}:{}", node.0), label)
        });
        TreeItem::new(format!("{MEMORY}:{ix}"), var.name.clone())
            .expanded(true)
            .children(uses)
    });
    items.push(
        TreeItem::new(MEMORY, t("tree-memory"))
            .expanded(true)
            .children(memory),
    );
    items
}

/// The index of the memory variable a tree id names.
fn memory_ix(id: &str) -> Option<usize> {
    id.strip_prefix(MEMORY)?.strip_prefix(':')?.parse().ok()
}

/// The graph and node a memory variable's use names.
fn memory_use(id: &str) -> Option<(usize, NodeId)> {
    let mut parts = id.strip_prefix("use:")?.splitn(3, ':').skip(1);
    let graph = parts.next()?.parse().ok()?;
    Some((graph, NodeId(parts.next()?.to_owned())))
}

/// The events "+" by Events offers, by name (D-092).
fn event_menu(state: &Entity<AppState>, mut menu: PopupMenu, cx: &App) -> PopupMenu {
    let mut events: Vec<(String, String)> = state
        .read(cx)
        .registry
        .iter()
        .filter(|t| t.category == Category::Trigger && t.id != "core.slash_command")
        .map(|t| (node_name(&t.id), t.id.clone()))
        .collect();
    events.sort();
    for (name, id) in events {
        let state = state.clone();
        menu = menu
            .item(PopupMenuItem::new(name).on_click(move |_, _, cx| add_event(&state, &id, cx)));
    }
    menu
}

/// The right-click menu of a graph, "Delete" after asking, or of a memory variable,
/// "Rename" and "Delete" (D-109).
fn item_menu(
    state: &Entity<AppState>,
    this: &WeakEntity<ProjectTree>,
    item: &TreeItem,
    menu: PopupMenu,
) -> PopupMenu {
    let (state, name) = (state.clone(), item.label.to_string());
    if let Ok(graph) = item.id.parse::<usize>() {
        return menu.item(
            PopupMenuItem::new(t("delete")).on_click(move |_, window, cx| {
                let delete = move |state: &mut AppState, cx: &mut Context<'_, AppState>| {
                    state.delete_graph(graph, cx);
                };
                confirm_delete(&state, &name, "delete-graph-detail", delete, window, cx);
            }),
        );
    }
    let Some(var) = memory_ix(&item.id) else {
        return menu;
    };
    let this = this.clone();
    menu.item(
        PopupMenuItem::new(t("rename")).on_click(move |_, window, cx| {
            let _ = this.update(cx, |this, cx| {
                this.start_adding(Adding::Rename(var), window, cx);
            });
        }),
    )
    .item(
        PopupMenuItem::new(t("delete")).on_click(move |_, window, cx| {
            let delete = move |state: &mut AppState, cx: &mut Context<'_, AppState>| {
                state.delete_memory(var, cx);
            };
            confirm_delete(&state, &name, "delete-memory-detail", delete, window, cx);
        }),
    )
}

impl Render for ProjectTree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let state = self.state.clone();
        let this = cx.entity().downgrade();
        let tree = tree(&self.tree, move |ix, entry, selected, _, _| {
            let item = entry.item();
            let mut row = ListItem::new(ix)
                .selected(selected)
                .pl(px(8.
                    + 12.
                        * f32::from(
                            u8::try_from(entry.depth()).unwrap_or(u8::MAX),
                        )))
                .child(item.label.clone());
            if let Ok(graph) = item.id.parse::<usize>() {
                let state = state.clone();
                row = row.on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.open_graph = Some(graph);
                        state.selected_node = None;
                        cx.notify();
                    });
                });
            } else if let Some((graph, node)) = memory_use(&item.id) {
                let state = state.clone();
                row = row.on_click(move |_, _, cx| {
                    let node = node.clone();
                    state.update(cx, |state, cx| state.show_node(graph, Some(node), cx));
                });
            } else if item.id.as_ref() == GraphKind::Command.dir() {
                let this = this.clone();
                row = row.suffix(move |_, _| {
                    let this = this.clone();
                    Button::new("add-command")
                        .ghost()
                        .xsmall()
                        .label("+")
                        .on_click(move |_, window, cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.start_adding(Adding::Command, window, cx);
                            });
                        })
                });
            } else if item.id.as_ref() == GraphKind::Event.dir() {
                let state = state.clone();
                row = row.suffix(move |_, _| {
                    let state = state.clone();
                    Button::new("add-event")
                        .ghost()
                        .xsmall()
                        .label("+")
                        .dropdown_menu(move |menu, _, cx| event_menu(&state, menu, cx))
                });
            }
            row
        })
        .context_menu({
            let (state, this) = (self.state.clone(), cx.entity().downgrade());
            move |_, entry, menu, _, _| item_menu(&state, &this, entry.item(), menu)
        })
        .size_full();
        let adding = self.adding.as_ref().map(|(_, input, _)| {
            v_flex()
                .p(px(6.))
                .gap(px(4.))
                .child(Input::new(input).small())
                .children(
                    self.error
                        .clone()
                        .map(|error| div().text_xs().text_color(cx.theme().danger).child(error)),
                )
        });
        v_flex()
            .size_full()
            .children(adding)
            .child(div().flex_1().min_h_0().child(tree))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{AppContext as _, TestAppContext};
    use rustbot_core::{GraphKind, NodeId};

    use super::{Adding, ProjectTree};
    use crate::state::AppState;

    /// "+" by Commands asks for a name, refuses a bad or taken one, and adds and opens the
    /// command (D-092).
    #[gpui_kit::test]
    fn plus_adds_a_named_command(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        let root = project.root.clone();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (tree, cx) = cx.add_window_view(|_, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            state = Some(app.clone());
            ProjectTree::new(app, cx)
        });
        let state = state.unwrap();
        for (name, error) in [("Ban", true), ("ping", true), ("ban", false)] {
            cx.update(|window, cx| window.click("add-command", cx));
            cx.update(|window, cx| {
                window.input(name, cx);
                window.press("enter", cx);
            });
            let shown = cx.read(|cx| tree.read(cx).error.is_some());
            assert_eq!(shown, error, "{name}");
            if error {
                cx.update(|_, cx| tree.update(cx, |tree, _| tree.adding = None));
            }
        }
        cx.read(|cx| {
            let state = state.read(cx);
            let file = state.open_file().unwrap();
            assert_eq!((file.kind, file.name.as_str()), (GraphKind::Command, "ban"));
            let trigger = &file.graph.nodes[&NodeId("n1".to_owned())];
            assert_eq!(
                (trigger.kind.as_str(), &trigger.props["name"]),
                ("core.slash_command", &"ban".into())
            );
        });
        assert!(root.join("graphs/commands/ban.json").exists());
    }

    /// Events are named after themselves; deleting a graph keeps the others' undo (D-092).
    #[gpui_kit::test]
    fn events_get_free_names_and_deleting_keeps_undo(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        let root = project.root.clone();
        let state = cx.new(|cx| AppState::new(project, cx));
        cx.update(|cx| {
            super::add_event(&state, "core.member_join", cx);
            super::add_event(&state, "core.member_join", cx);
            state.update(cx, |state, cx| {
                let names: Vec<_> = state
                    .project
                    .graphs
                    .iter()
                    .map(|f| f.name.clone())
                    .collect();
                assert_eq!(names, ["ping", "member_join", "member_join_2"]);
                // An undoable change to member_join_2, then ping goes.
                state.edit(true, cx, |file, registry| {
                    crate::canvas::add_node(file, registry, "core.delete_message", [0., 0.]);
                });
                state.delete_graph(0, cx);
                assert_eq!(state.project.graphs.len(), 2);
                assert_eq!(state.project.graphs[1].graph.nodes.len(), 2);
                state.undo(false, cx);
                assert_eq!(state.project.graphs[1].graph.nodes.len(), 1);
                assert_eq!(state.open_graph, Some(1));
            });
        });
        assert!(!root.join("graphs/commands/ping.json").exists());
        assert!(root.join("graphs/events/member_join_2.json").exists());
    }

    /// The tree lists each memory variable with the nodes that use it, and renaming it
    /// fixes them, saved (D-109).
    #[gpui_kit::test]
    fn memory_lists_its_uses_and_renames_them(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        let root = project.root.clone();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (tree, cx) = cx.add_window_view(|_, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, cx| {
                app.add_memory("warnings", rustbot_core::MemoryType::Number, cx);
                app.add_memory("other", rustbot_core::MemoryType::Number, cx);
                app.edit(false, cx, |file, registry| {
                    let id =
                        crate::canvas::add_node(file, registry, "core.memory_change", [0., 0.]);
                    let change = file.graph.nodes.get_mut(&id).unwrap();
                    change
                        .props
                        .insert("variable".to_owned(), "warnings".into());
                    let reply = file.graph.nodes.get_mut(&NodeId("n2".to_owned())).unwrap();
                    reply.props.insert(
                        "content".to_owned(),
                        "{{memory.warnings}} {memory.warnings.option.x} {memory.other}".into(),
                    );
                });
                app.save_now();
            });
            state = Some(app.clone());
            ProjectTree::new(app, cx)
        });
        let state = state.unwrap();
        let ids = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                super::items(&state.read(cx).project)[2]
                    .children
                    .iter()
                    .map(|var| (var.id.to_string(), var.children.len()))
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(
            ids(cx),
            [("memory:0".to_owned(), 2), ("memory:1".to_owned(), 1)]
        );
        assert_eq!(
            super::memory_use("use:0:0:n3"),
            Some((0, NodeId("n3".to_owned())))
        );

        for (name, error) in [("other", true), ("Strikes", true), ("strikes", false)] {
            cx.update(|window, cx| {
                tree.update(cx, |tree, cx| {
                    tree.start_adding(Adding::Rename(0), window, cx);
                });
                let input = tree.read(cx).adding.as_ref().unwrap().1.clone();
                input.update(cx, |input, cx| input.set_value(name, window, cx));
                window.press("enter", cx);
            });
            assert_eq!(cx.read(|cx| tree.read(cx).error.is_some()), error, "{name}");
        }
        cx.read(|cx| {
            let file = &state.read(cx).project.graphs[0];
            let node = |id: &str| &file.graph.nodes[&NodeId(id.to_owned())].props;
            assert_eq!(node("n3")["variable"], "strikes");
            assert_eq!(
                node("n2")["content"],
                "{{memory.warnings}} {memory.strikes.option.x} {memory.other}"
            );
        });
        let saved = std::fs::read_to_string(root.join("graphs/commands/ping.json")).unwrap();
        assert!(saved.contains("memory.strikes"), "{saved}");
        let toml = std::fs::read_to_string(root.join("project.toml")).unwrap();
        assert!(toml.contains("name = \"strikes\""), "{toml}");

        cx.update(|_, cx| state.update(cx, |state, cx| state.delete_memory(0, cx)));
        assert_eq!(ids(cx), [("memory:0".to_owned(), 1)]);
    }
}
