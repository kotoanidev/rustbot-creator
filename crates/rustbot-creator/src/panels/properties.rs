//! The selected node's form (D-066). The fields come with the graph editor.

use gpui_kit::{Context, Entity, FocusHandle, IntoElement, Render, Window};

use crate::i18n::t;
use crate::state::AppState;

pub(crate) struct Properties {
    focus: FocusHandle,
    _state: Entity<AppState>,
}

super::dock_panel!(Properties, "Properties", "panel-properties");

impl Properties {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            _state: state,
        }
    }
}

impl Render for Properties {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        super::placeholder(t("properties-empty"), cx)
    }
}
