//! The bot's and the build's output, in the bottom panel.

use gpui_kit::{
    Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use crate::i18n::t;
use crate::state::AppState;

pub(crate) struct Logs {
    focus: FocusHandle,
    state: Entity<AppState>,
}

super::dock_panel!(Logs, "Logs", "panel-logs");

impl Logs {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            state,
        }
    }
}

impl Render for Logs {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let logs = &self.state.read(cx).logs;
        if logs.is_empty() {
            return super::placeholder(t("logs-empty"), cx).into_any_element();
        }
        // shortcut: every line is an element; switch to a virtual list once long runs lag.
        div()
            .id("logs")
            .size_full()
            .overflow_y_scroll()
            .p(px(8.))
            .font_family("monospace")
            .text_xs()
            .children(logs.iter().map(|line| div().child(line.clone())))
            .into_any_element()
    }
}
