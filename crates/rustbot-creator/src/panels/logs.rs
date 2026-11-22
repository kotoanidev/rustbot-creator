//! The bot's and the build's output, in the bottom panel.

use gpui_kit::{
    Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use crate::i18n::t;
use crate::state::AppState;

pub(crate) struct Logs {
    focus: FocusHandle,
    state: Entity<AppState>,
    scroll: ScrollHandle,
    /// How many lines the last frame showed, to follow new ones.
    shown: usize,
}

super::dock_panel!(Logs, "Logs", "panel-logs");

impl Logs {
    pub(crate) fn new(state: Entity<AppState>, cx: &mut Context<'_, Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self {
            focus: cx.focus_handle(),
            state,
            scroll: ScrollHandle::new(),
            shown: 0,
        }
    }
}

impl Render for Logs {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let logs = &self.state.read(cx).logs;
        if logs.is_empty() {
            return super::placeholder(t("logs-empty"), cx).into_any_element();
        }
        if logs.len() != self.shown {
            self.shown = logs.len();
            self.scroll.scroll_to_bottom();
        }
        // shortcut: every line is an element; switch to a virtual list once long runs lag.
        div()
            .id("logs")
            .track_scroll(&self.scroll)
            .size_full()
            .overflow_y_scroll()
            .p(px(8.))
            .font_family("monospace")
            .text_xs()
            .children(logs.iter().map(|line| div().child(line.clone())))
            .into_any_element()
    }
}
