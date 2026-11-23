//! The bot's and the build's output, in the bottom panel.

use gpui_kit::component::link::Link;
use gpui_kit::{
    AnyElement, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
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
            .children(
                logs.iter()
                    .enumerate()
                    .map(|(ix, line)| render_line(ix, line)),
            )
            .into_any_element()
    }
}

/// A line, or a link that opens in the browser when the line is only a web address, like
/// the invite link a bot prints when it is not on its test server (D-072).
// shortcut: only whole-line links are clickable; find links inside lines when bots print them.
fn render_line(ix: usize, line: &str) -> AnyElement {
    match link(line) {
        Some(address) => Link::new(("log-link", ix))
            .href(address.to_owned())
            .child(address.to_owned())
            .into_any_element(),
        None => div().child(line.to_owned()).into_any_element(),
    }
}

/// The web address a line consists of, if it is nothing else.
fn link(line: &str) -> Option<&str> {
    let address = line.trim();
    (address.starts_with("https://") && !address.contains(char::is_whitespace)).then_some(address)
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_a_line_that_is_a_web_address_is_a_link() {
        let url = "https://discord.com/oauth2/authorize?client_id=1&scope=bot";
        assert_eq!(super::link(&format!("  {url} ")), Some(url));
        assert_eq!(super::link(&format!("invite: {url}")), None);
        assert_eq!(super::link("http://example.com"), None);
        assert_eq!(super::link("Ready as bot#0001"), None);
    }
}
