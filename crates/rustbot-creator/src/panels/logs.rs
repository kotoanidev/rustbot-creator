//! The bot's and the build's output, in the bottom panel.

use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::link::Link;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, Theme};
use gpui_kit::{
    AnyElement, Context, Entity, FocusHandle, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _,
    TextStyleRefinement, Window, div, px,
};

use crate::i18n::t;
use crate::state::{AppState, Level, Line, Source};

pub(crate) struct Logs {
    focus: FocusHandle,
    state: Entity<AppState>,
    scroll: ScrollHandle,
    /// How many lines the last frame showed, to follow new ones.
    shown: usize,
}

super::dock_panel!(
    Logs,
    "Logs",
    "panel-logs",
    fn toolbar_buttons(
        &mut self,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) -> Option<Vec<Button>> {
        let state = self.state.clone();
        Some(vec![
            Button::new("logs-clear")
                .ghost()
                .xsmall()
                .label(t("logs-clear"))
                .on_click(move |_, _, cx| {
                    state.update(cx, |state, cx| {
                        state.logs.clear();
                        cx.notify();
                    });
                }),
        ])
    }
);

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
            self.shown = 0;
            return super::placeholder(t("logs-empty"), cx).into_any_element();
        }
        if logs.len() != self.shown {
            self.shown = logs.len();
            self.scroll.scroll_to_bottom();
        }
        let theme = cx.theme();
        // shortcut: every line is an element; switch to a virtual list once long runs lag.
        div()
            .id("logs")
            // A click focuses the panel, so Ctrl+C reaches the window's copy of the selection.
            .track_focus(&self.focus)
            .track_scroll(&self.scroll)
            .size_full()
            .overflow_y_scroll()
            .p(px(8.))
            .font_family("monospace")
            .text_xs()
            .children(
                logs.iter()
                    .enumerate()
                    .map(|(ix, line)| render_line(ix, line, theme)),
            )
            .into_any_element()
    }
}

/// A line as selectable text, coloured by its level and source (D-075), or a link that
/// opens in the browser when the line is only a web address, like the invite link a bot
/// prints when it is not on its test server (D-072).
// shortcut: only whole-line links are clickable; find links inside lines when bots print them.
fn render_line(ix: usize, line: &Line, theme: &Theme) -> AnyElement {
    if let Some(address) = link(&line.text) {
        return Link::new(("log-link", ix))
            .href(address.to_owned())
            .child(address.to_owned())
            .into_any_element();
    }
    let color = match (line.level, line.source) {
        (Level::Error, _) => theme.danger,
        (Level::Warn, _) => theme.warning,
        // The build's output is quieter than the bot's, so the two are told apart.
        (Level::Info, Source::Build) => theme.muted_foreground,
        (Level::Info, Source::App | Source::Bot) => theme.foreground,
    };
    // The app's own lines, such as "Building the bot…", mark where each step starts.
    let font_weight = (line.source == Source::App).then_some(FontWeight::SEMIBOLD);
    // Each line is its own run; the window's selection spans runs in document order.
    SelectableText::new(("log", ix), line.text.clone())
        .document_order(ix as u64)
        .text_style(TextStyleRefinement {
            color: Some(color),
            font_weight,
            ..TextStyleRefinement::default()
        })
        .into_any_element()
}

/// The web address a line consists of, if it is nothing else.
fn link(line: &str) -> Option<&str> {
    let address = line.trim();
    (address.starts_with("https://") && !address.contains(char::is_whitespace)).then_some(address)
}

#[cfg(test)]
mod tests {
    use gpui_kit::base::Root;
    use gpui_kit::{AppContext as _, Modifiers, MouseButton, TestAppContext, point};

    use super::*;

    /// A drag across two lines selects both, and Ctrl+C copies them, through the window
    /// root the app opens (D-075).
    #[gpui_kit::test]
    fn a_selection_spans_lines_and_copies(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| AppState::new(project, cx));
            state.update(cx, |state, _| {
                state.log("Building the bot".to_owned());
                state.push(Line::output(Source::Bot, "Ready as bot".to_owned(), false));
            });
            let logs = cx.new(|cx| Logs::new(state, cx));
            Root::new(logs, window, cx)
        });
        draw(cx);
        // From the first character of line 1 to past the end of line 2.
        cx.simulate_mouse_down(
            point(px(9.), px(12.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        let end = point(px(400.), px(30.));
        cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
        cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
        draw(cx);
        cx.simulate_keystrokes("ctrl-c");
        let copied = cx.read_from_clipboard().and_then(|item| item.text());
        assert_eq!(copied.as_deref(), Some("Building the bot\nReady as bot"));
    }

    fn draw(cx: &mut gpui_kit::VisualTestContext) {
        cx.update(|window, cx| {
            let _ = window.draw(cx);
        });
    }

    #[test]
    fn only_a_line_that_is_a_web_address_is_a_link() {
        let url = "https://discord.com/oauth2/authorize?client_id=1&scope=bot";
        assert_eq!(super::link(&format!("  {url} ")), Some(url));
        assert_eq!(super::link(&format!("invite: {url}")), None);
        assert_eq!(super::link("http://example.com"), None);
        assert_eq!(super::link("Ready as bot#0001"), None);
    }
}
