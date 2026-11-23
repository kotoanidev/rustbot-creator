//! The side bar's form (D-066): the selected node's fields, or the bot's settings when no
//! node is selected: its token, kept in the system keyring (D-020), and the test server
//! (D-055, D-065).

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::v_flex;
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render,
    Styled as _, Subscription, Window, div, px,
};
use rustbot_core::registry::PropKind;
use rustbot_core::{GraphFile, NodeId, Registry};

use crate::i18n::{t, t_or, t_with};
use crate::state::AppState;

pub(crate) struct Properties {
    focus: FocusHandle,
    state: Entity<AppState>,
    /// The node the fields belong to: `(graph index, node id)`.
    shown: Option<(usize, NodeId)>,
    fields: Vec<Field>,
    /// The next change starts a new undo step: set when a field gains the focus.
    fresh: bool,
    bot: Option<BotForm>,
}

super::dock_panel!(Properties, "Properties", "panel-properties");

struct Field {
    prop: String,
    templates: bool,
    input: Entity<InputState>,
    _changes: Subscription,
}

struct BotForm {
    token: Entity<InputState>,
    server: Entity<InputState>,
    /// What the form last found or did, e.g. "a token is saved"; `true` for an error.
    token_status: (String, bool),
    server_status: Option<(String, bool)>,
    _changes: [Subscription; 2],
}

impl Properties {
    pub(crate) fn new(
        state: Entity<AppState>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        cx.observe_in(&state, window, |this, _, window, cx| this.sync(window, cx))
            .detach();
        let mut this = Self {
            focus: cx.focus_handle(),
            state,
            shown: None,
            fields: Vec::new(),
            fresh: true,
            bot: None,
        };
        this.sync(window, cx);
        this
    }

    /// Rebuilds the fields when another node is selected, and refreshes their text after
    /// an undo.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let state = self.state.read(cx);
        let target = state
            .open_graph
            .zip(state.selected_node.clone())
            .filter(|(_, node)| {
                state
                    .open_file()
                    .is_some_and(|file| file.graph.nodes.contains_key(node))
            });
        if target != self.shown {
            self.shown = target;
            self.build_fields(window, cx);
        } else if let (Some((_, node)), Some(file)) = (&self.shown, state.open_file()) {
            let values: Vec<String> = self
                .fields
                .iter()
                .map(|field| prop_text(file, node, &field.prop))
                .collect();
            for (field, value) in self.fields.iter().zip(values) {
                if field.input.read(cx).value() != value.as_str() {
                    field
                        .input
                        .update(cx, |input, cx| input.set_value(value, window, cx));
                }
            }
        }
        if self.shown.is_none() && self.bot.is_none() {
            self.bot = Some(self.build_bot_form(window, cx));
        }
        cx.notify();
    }

    fn build_fields(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.fields.clear();
        let Some((_, node)) = self.shown.clone() else {
            return;
        };
        let state = self.state.read(cx);
        let Some(file) = state.open_file() else {
            return;
        };
        let Some(kind) = state.registry.get(&file.graph.nodes[&node].kind) else {
            return;
        };
        let props: Vec<(String, bool, String)> = kind
            .props
            .iter()
            .filter_map(|prop| match prop.kind {
                PropKind::Text { templates } => Some((
                    prop.name.clone(),
                    templates,
                    prop_text(file, &node, &prop.name),
                )),
                // shortcut: the options list gets its editor in the next commit (D-076).
                PropKind::Options => None,
            })
            .collect();
        for (prop, templates, value) in props {
            let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
            let name = prop.clone();
            let changes =
                cx.subscribe_in(
                    &input,
                    window,
                    move |this, input, event, _, cx| match event {
                        InputEvent::Focus => this.fresh = true,
                        InputEvent::Change => {
                            let value = input.read(cx).value().to_string();
                            this.write(&name, value, cx);
                        }
                        _ => {}
                    },
                );
            self.fields.push(Field {
                prop,
                templates,
                input,
                _changes: changes,
            });
        }
    }

    /// Writes a field into the graph; the first change after focusing is one undo step.
    fn write(&mut self, prop: &str, value: String, cx: &mut Context<'_, Self>) {
        let Some((_, node)) = self.shown.clone() else {
            return;
        };
        let undoable = std::mem::take(&mut self.fresh);
        self.state.update(cx, |state, cx| {
            state.edit(undoable, cx, |file, _| {
                if let Some(node) = file.graph.nodes.get_mut(&node) {
                    node.props
                        .insert(prop.to_owned(), serde_json::Value::String(value));
                }
            });
        });
    }

    fn build_bot_form(&self, window: &mut Window, cx: &mut Context<'_, Self>) -> BotForm {
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder(t("bot-token-placeholder"))
        });
        let state = self.state.read(cx);
        let server_id = self
            .bot_dir(cx)
            .and_then(|dir| rustbot_runner::test_server(&dir).ok().flatten())
            .unwrap_or_default();
        let project = state.project.manifest.id;
        let server = cx.new(|cx| InputState::new(window, cx).default_value(server_id));
        let token_changes = cx.subscribe_in(&token, window, |this, _, event, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.save_token(window, cx);
            }
        });
        let server_changes = cx.subscribe_in(&server, window, |this, _, event, _, cx| {
            if let InputEvent::PressEnter { .. } | InputEvent::Blur = event {
                this.save_server(cx);
            }
        });
        // Whether a token is stored; the keyring may be slow or ask to be unlocked.
        let check = cx.background_spawn(async move { rustbot_secrets::bot_token(project) });
        cx.spawn(async move |this, cx| {
            let found = check.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(bot) = &mut this.bot {
                    bot.token_status = token_status(found.map(|token| token.is_some()));
                    cx.notify();
                }
            });
        })
        .detach();
        BotForm {
            token,
            server,
            token_status: (t("bot-token-checking"), false),
            server_status: None,
            _changes: [token_changes, server_changes],
        }
    }

    fn bot_dir(&self, cx: &gpui_kit::App) -> Option<std::path::PathBuf> {
        let state = self.state.read(cx);
        Some(state.toolchain.as_ref()?.bot_dir(&state.project))
    }

    /// Stores the typed token in the keyring and clears the field, so it is not kept in
    /// the window.
    fn save_token(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(bot) = &self.bot else { return };
        let token = bot.token.read(cx).value().trim().to_owned();
        if token.is_empty() {
            return;
        }
        bot.token
            .update(cx, |input, cx| input.set_value("", window, cx));
        let project = self.state.read(cx).project.manifest.id;
        let save =
            cx.background_spawn(async move { rustbot_secrets::set_bot_token(project, &token) });
        cx.spawn(async move |this, cx| {
            let saved = save.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(bot) = &mut this.bot {
                    bot.token_status = token_status(saved.map(|()| true));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn save_server(&mut self, cx: &mut Context<'_, Self>) {
        let Some(dir) = self.bot_dir(cx) else { return };
        let Some(bot) = &mut self.bot else { return };
        let id = bot.server.read(cx).value().trim().to_owned();
        let result =
            rustbot_runner::set_test_server(&dir, Some(id.as_str()).filter(|id| !id.is_empty()));
        bot.server_status = Some(match result {
            Ok(()) => (t("bot-test-server-saved"), false),
            Err(rustbot_runner::Error::InvalidServerId(id)) => {
                (t_with("error-server-id", &[("id", &id)]), true)
            }
            Err(e) => (e.to_string(), true),
        });
        cx.notify();
    }

    fn render_bot_form(bot: &BotForm, cx: &Context<'_, Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let status = |(text, error): (String, bool)| {
            div()
                .text_xs()
                .text_color(if error {
                    theme.danger
                } else {
                    theme.muted_foreground
                })
                .child(text)
        };
        v_flex()
            .gap(px(6.))
            .child(div().text_sm().child(t("bot-settings")))
            .child(label(t("bot-token")))
            .child(Input::new(&bot.token))
            .child(
                Button::new("save-token")
                    .primary()
                    .label(t("save"))
                    .on_click(cx.listener(|this, _, window, cx| this.save_token(window, cx))),
            )
            .child(status(bot.token_status.clone()))
            .child(label(t("bot-test-server")))
            .child(Input::new(&bot.server))
            .child(status((t("bot-test-server-hint"), false)))
            .children(bot.server_status.clone().map(status))
    }
}

fn token_status(found: Result<bool, rustbot_secrets::Error>) -> (String, bool) {
    match found {
        Ok(true) => (t("bot-token-saved"), false),
        Ok(false) => (t("bot-token-missing"), false),
        Err(e) => (t_with("error-keyring", &[("error", &e.to_string())]), true),
    }
}

fn label(text: String) -> impl IntoElement {
    div().pt(px(6.)).text_xs().child(text)
}

/// A field's text; anything that is not a string shows as JSON.
fn prop_text(file: &GraphFile, node: &NodeId, prop: &str) -> String {
    match file
        .graph
        .nodes
        .get(node)
        .and_then(|node| node.props.get(prop))
    {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// The variables a template in `file` can use, e.g. `{user.name}`, from its trigger.
fn variables(file: &GraphFile, registry: &Registry) -> Vec<String> {
    let mut found = Vec::new();
    for node in file.graph.nodes.values() {
        let Some(kind) = registry.get(&node.kind) else {
            continue;
        };
        for variable in &kind.provides {
            found.extend(
                variable
                    .ty
                    .fields()
                    .iter()
                    .map(|field| format!("{{{}.{field}}}", variable.name)),
            );
        }
    }
    found
}

impl Render for Properties {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.theme();
        let state = self.state.read(cx);
        let body = match (&self.shown, state.open_file()) {
            (Some((_, node)), Some(file)) => {
                let kind = &file.graph.nodes[node].kind;
                let node_id = kind.replace('.', "-");
                let hint = variables(file, &state.registry).join(" ");
                v_flex()
                    .gap(px(6.))
                    .child(div().text_sm().child(super::editor::node_name(kind)))
                    .children(self.fields.iter().map(|field| {
                        let name = t_or(&format!("prop-{node_id}-{}", field.prop), &field.prop);
                        v_flex()
                            .gap(px(4.))
                            .child(label(name))
                            .child(Input::new(&field.input))
                            .children((field.templates && !hint.is_empty()).then(|| {
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(t_with("props-variables", &[("list", &hint)]))
                            }))
                    }))
                    .into_any_element()
            }
            _ => match &self.bot {
                Some(bot) => Self::render_bot_form(bot, cx).into_any_element(),
                None => super::placeholder(t("properties-empty"), cx).into_any_element(),
            },
        };
        div().size_full().p(px(12.)).child(body)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{AppContext as _, TestAppContext};
    use rustbot_core::NodeId;

    use super::Properties;
    use crate::state::AppState;

    /// Typing into the reply's field changes the graph; Ctrl+Z in the canvas puts the old
    /// text back into the field too.
    #[gpui_kit::test]
    fn a_field_edits_the_node_and_follows_undo(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, _| {
                app.selected_node = Some(NodeId("n2".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        cx.update(|window, cx| {
            let field = properties.read(cx).fields[0].input.clone();
            field.update(cx, |input, cx| input.focus(window, cx));
            window.render_frame(cx);
        });
        cx.update(|window, cx| window.press("end", cx));
        cx.update(|window, cx| window.input(" Hi", cx));
        let content = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                file.graph.nodes[&NodeId("n2".to_owned())].props["content"].clone()
            })
        };
        assert_eq!(content(cx), "Pong, {user.name}! Hi");

        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.undo(false, cx));
            window.render_frame(cx);
        });
        assert_eq!(content(cx), "Pong, {user.name}!");
        let shown = cx.read(|cx| properties.read(cx).fields[0].input.read(cx).value());
        assert_eq!(shown.as_str(), "Pong, {user.name}!");
    }
}
