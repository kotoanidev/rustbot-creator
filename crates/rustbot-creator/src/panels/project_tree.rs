//! The project tree in the side bar: the project's commands and events.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _,
    PromptLevel, Render, Styled as _, Subscription, Window, div, px,
};
use rustbot_core::registry::{Category, valid_command_name};
use rustbot_core::{GraphKind, Project};

use crate::i18n::{t, t_with};
use crate::panels::editor::node_name;
use crate::state::AppState;

pub(crate) struct ProjectTree {
    focus: FocusHandle,
    state: Entity<AppState>,
    tree: Entity<TreeState>,
    /// The name field of a command being added (D-092).
    adding: Option<(Entity<InputState>, Subscription)>,
    error: Option<String>,
}

super::dock_panel!(ProjectTree, "ProjectTree", "panel-project");

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

    /// Shows the name field for a new command.
    fn start_adding(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("new-command-placeholder")));
        input.update(cx, |input, cx| input.focus(window, cx));
        let changes = cx.subscribe_in(&input, window, |this, input, event, _, cx| match event {
            InputEvent::PressEnter { .. } => {
                let name = input.read(cx).value().trim().to_owned();
                this.add_command(&name, cx);
            }
            // Leaving an empty field gives up.
            InputEvent::Blur if input.read(cx).value().trim().is_empty() => {
                this.adding = None;
                this.error = None;
                cx.notify();
            }
            _ => {}
        });
        self.adding = Some((input, changes));
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

/// Adds a graph for the event `trigger`, named after it (D-092).
fn add_event(state: &Entity<AppState>, trigger: &str, cx: &mut App) {
    state.update(cx, |state, cx| {
        let name = state.free_name(GraphKind::Event, trigger.trim_start_matches("core."));
        if let Err(e) = state.add_graph(GraphKind::Event, &name, trigger, cx) {
            state.log_error(e.to_string());
        }
    });
}

/// Asks before deleting graph `graph`, which cannot be undone (D-092).
fn confirm_delete(
    state: &Entity<AppState>,
    graph: usize,
    name: &str,
    window: &mut Window,
    cx: &mut App,
) {
    let (delete, cancel) = (t("delete"), t("cancel"));
    let answer = window.prompt(
        PromptLevel::Warning,
        &t_with("delete-graph-title", &[("name", name)]),
        Some(&t("delete-graph-detail")),
        &[delete.as_str(), cancel.as_str()],
        cx,
    );
    let state = state.clone();
    window
        .spawn(cx, async move |cx| {
            if answer.await == Ok(0) {
                let _ =
                    cx.update(|_, cx| state.update(cx, |state, cx| state.delete_graph(graph, cx)));
            }
        })
        .detach();
}

/// A folder per graph kind; a graph's id is its index in `project.graphs`.
fn items(project: &Project) -> Vec<TreeItem> {
    [
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
    .collect()
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
            } else if item.id.as_ref() == GraphKind::Command.dir() {
                let this = this.clone();
                row = row.suffix(move |_, _| {
                    let this = this.clone();
                    Button::new("add-command")
                        .ghost()
                        .xsmall()
                        .label("+")
                        .on_click(move |_, window, cx| {
                            let _ = this.update(cx, |this, cx| this.start_adding(window, cx));
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
                        .dropdown_menu(move |mut menu, _, cx| {
                            let mut events: Vec<(String, String)> = state
                                .read(cx)
                                .registry
                                .iter()
                                .filter(|t| {
                                    t.category == Category::Trigger && t.id != "core.slash_command"
                                })
                                .map(|t| (node_name(&t.id), t.id.clone()))
                                .collect();
                            events.sort();
                            for (name, id) in events {
                                let state = state.clone();
                                menu = menu.item(
                                    PopupMenuItem::new(name)
                                        .on_click(move |_, _, cx| add_event(&state, &id, cx)),
                                );
                            }
                            menu
                        })
                });
            }
            row
        })
        .context_menu({
            let state = self.state.clone();
            move |_, entry, menu, _, _| {
                let item = entry.item();
                let Ok(graph) = item.id.parse::<usize>() else {
                    return menu;
                };
                let (state, name) = (state.clone(), item.label.to_string());
                menu.item(
                    PopupMenuItem::new(t("delete")).on_click(move |_, window, cx| {
                        confirm_delete(&state, graph, &name, window, cx);
                    }),
                )
            }
        })
        .size_full();
        let adding = self.adding.as_ref().map(|(input, _)| {
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

    use super::ProjectTree;
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
}
