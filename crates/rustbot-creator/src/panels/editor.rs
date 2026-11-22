//! The centre: the open graph. The canvas comes in the next step (D-042).

use gpui_kit::{Context, Entity, FocusHandle, IntoElement, Render, Window};

use crate::i18n::t;
use crate::state::AppState;

pub(crate) struct Editor {
    focus: FocusHandle,
    state: Entity<AppState>,
}

super::dock_panel!(Editor, "Editor", "panel-editor");

impl Editor {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            state,
        }
    }
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let open = state.open_graph.and_then(|ix| state.project.graphs.get(ix));
        let text = open.map_or_else(|| t("editor-empty"), |file| file.name.clone());
        super::placeholder(text, cx)
    }
}
