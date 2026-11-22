//! The project tree in the side bar: the project's commands and events.

use gpui_kit::component::list::ListItem;
use gpui_kit::component::tree::{TreeItem, TreeState, tree};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render,
    Styled as _, Window, px,
};
use rustbot_core::{GraphKind, Project};

use crate::i18n::t;
use crate::state::AppState;

pub(crate) struct ProjectTree {
    focus: FocusHandle,
    state: Entity<AppState>,
    tree: Entity<TreeState>,
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
        }
    }
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
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let state = self.state.clone();
        tree(&self.tree, move |ix, entry, selected, _, _| {
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
            }
            row
        })
        .size_full()
    }
}
