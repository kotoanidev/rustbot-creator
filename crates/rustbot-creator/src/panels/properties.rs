//! The side bar's form (D-066): the selected node's fields, or the bot's settings when no
//! node is selected: its token, kept in the system keyring (D-020), and the test server
//! (D-055, D-065).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{ActiveTheme as _, IndexPath, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, div, px,
};
use rustbot_core::registry::{CommandOption, DataType, PropKind, is_id};
use rustbot_core::{GraphFile, NodeId, Registry};

use crate::i18n::{t, t_or, t_with};
use crate::state::AppState;

pub(crate) struct Properties {
    focus: FocusHandle,
    state: Entity<AppState>,
    /// The node the fields belong to: `(graph index, node id)`.
    shown: Option<(usize, NodeId)>,
    fields: Vec<Field>,
    pickers: Vec<Picker>,
    /// The form's fields in the node type's order.
    layout: Vec<Slot>,
    /// The prop holding the shown node's command options, if it has one (D-076).
    options_prop: Option<String>,
    options: Vec<OptionRow>,
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

/// Where a form field is kept.
#[derive(Clone, Copy)]
enum Slot {
    Text(usize),
    Pick(usize),
    Options,
}

/// A field picked from a list: a variable of a type (D-078), or one value of a fixed list.
struct Picker {
    prop: String,
    /// What each entry writes into the graph, in list order.
    values: Vec<String>,
    labels: Vec<SharedString>,
    /// A role field ends its list with "Role with ID…", which shows `id` (D-080).
    id: Option<Entity<InputState>>,
    id_chosen: bool,
    select: Entity<SelectState<Vec<SharedString>>>,
    _changes: Vec<Subscription>,
}

/// The editable parts of one command option; its "required" box is drawn from the graph.
struct OptionRow {
    name: Entity<InputState>,
    description: Entity<InputState>,
    ty: Entity<SelectState<Vec<SharedString>>>,
    _changes: [Subscription; 3],
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
            pickers: Vec::new(),
            layout: Vec::new(),
            options_prop: None,
            options: Vec::new(),
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
            if !self.sync_pickers(window, cx) {
                self.build_fields(window, cx);
            }
            self.sync_options(window, cx);
        }
        if self.shown.is_none() && self.bot.is_none() {
            self.bot = Some(self.build_bot_form(window, cx));
        }
        cx.notify();
    }

    fn build_fields(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.fields.clear();
        self.pickers.clear();
        self.layout.clear();
        self.options_prop = None;
        self.options.clear();
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
        let props: Vec<(String, PropKind, String)> = kind
            .props
            .iter()
            .map(|prop| {
                (
                    prop.name.clone(),
                    prop.kind,
                    prop_text(file, &node, &prop.name),
                )
            })
            .collect();
        for (prop, kind, value) in props {
            match kind {
                PropKind::Text { .. } | PropKind::Number { .. } => {
                    // A number may come from a variable too (D-084).
                    let templates = kind != PropKind::Text { templates: false };
                    self.layout.push(Slot::Text(self.fields.len()));
                    let field = Self::text_field(prop, templates, value, window, cx);
                    self.fields.push(field);
                }
                PropKind::Variable(_) | PropKind::Choice(_) => {
                    self.layout.push(Slot::Pick(self.pickers.len()));
                    let picker = self.picker(prop, kind, &value, window, cx);
                    self.pickers.push(picker);
                }
                PropKind::Options => {
                    self.layout.push(Slot::Options);
                    self.options_prop = Some(prop);
                }
            }
        }
        self.build_options(window, cx);
    }

    fn text_field(
        prop: String,
        templates: bool,
        value: String,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Field {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let name = prop.clone();
        let changes = cx.subscribe_in(
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
        Field {
            prop,
            templates,
            input,
            _changes: changes,
        }
    }

    /// The entries a picker offers now: what each writes, and its label.
    fn entries(&self, kind: PropKind, cx: &gpui_kit::App) -> (Vec<String>, Vec<SharedString>) {
        let values: Vec<String> = match kind {
            PropKind::Variable(ty) => {
                let state = self.state.read(cx);
                state.open_file().map_or_else(Vec::new, |file| {
                    state
                        .registry
                        .scope(&file.graph)
                        .into_iter()
                        .filter(|var| var.ty == ty)
                        .map(|var| var.name)
                        .collect()
                })
            }
            PropKind::Choice(choices) => choices.values().iter().map(|&v| v.to_owned()).collect(),
            PropKind::Text { .. } | PropKind::Options | PropKind::Number { .. } => Vec::new(),
        };
        let mut labels: Vec<SharedString> = values
            .iter()
            .map(|value| match kind {
                PropKind::Variable(_) => variable_label(value).into(),
                PropKind::Choice(choices) => choice_label(choices, value).into(),
                PropKind::Text { .. } | PropKind::Options | PropKind::Number { .. } => {
                    value.clone().into()
                }
            })
            .collect();
        if kind == PropKind::Variable(DataType::Role) {
            labels.push(t("var-role-id").into());
        }
        (values, labels)
    }

    fn picker(
        &self,
        prop: String,
        kind: PropKind,
        value: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Picker {
        let (values, labels) = self.entries(kind, cx);
        let takes_id = kind == PropKind::Variable(DataType::Role);
        let id_chosen = takes_id && is_id(value);
        let selected = if id_chosen {
            Some(values.len())
        } else {
            values.iter().position(|v| v == value)
        };
        let select =
            cx.new(|cx| SelectState::new(labels.clone(), selected.map(IndexPath::new), window, cx));
        let ix = self.pickers.len();
        let mut changes =
            vec![
                cx.subscribe_in(&select, window, move |this, _, event, window, cx| {
                    if let SelectEvent::Confirm(Some(label)) = event {
                        let entry = this.pickers[ix].labels.iter().position(|l| l == label);
                        if let Some(entry) = entry {
                            this.pick(ix, entry, window, cx);
                        }
                    }
                }),
            ];
        let id = takes_id.then(|| {
            let shown = if id_chosen {
                value.to_owned()
            } else {
                String::new()
            };
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(shown)
                    .placeholder(t("role-id-placeholder"))
            });
            let name = prop.clone();
            changes.push(
                cx.subscribe_in(
                    &input,
                    window,
                    move |this, input, event, _, cx| match event {
                        InputEvent::Focus => this.fresh = true,
                        InputEvent::Change => {
                            let value = input.read(cx).value().trim().to_owned();
                            this.write(&name, value, cx);
                        }
                        _ => {}
                    },
                ),
            );
            input
        });
        Picker {
            prop,
            values,
            labels,
            id,
            id_chosen,
            select,
            _changes: changes,
        }
    }

    /// Picker `ix` took entry `entry`: a value, or "Role with ID…", which writes the typed
    /// ID. Each pick is an undo step of its own.
    fn pick(&mut self, ix: usize, entry: usize, window: &mut Window, cx: &mut Context<'_, Self>) {
        let Some(picker) = self.pickers.get_mut(ix) else {
            return;
        };
        picker.id_chosen = entry >= picker.values.len();
        let value = if let Some(value) = picker.values.get(entry) {
            value.clone()
        } else {
            let id = picker.id.clone();
            id.map_or_else(String::new, |id| {
                id.update(cx, |input, cx| input.focus(window, cx));
                id.read(cx).value().trim().to_owned()
            })
        };
        let prop = self.pickers[ix].prop.clone();
        self.fresh = true;
        self.write(&prop, value, cx);
    }

    /// Brings the pickers up to date after an undo. `false` when their lists changed, e.g. a
    /// command option was added, and the form must be built again.
    fn sync_pickers(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> bool {
        let Some((_, node)) = self.shown.clone() else {
            return true;
        };
        let kinds: Vec<Option<PropKind>> = {
            let state = self.state.read(cx);
            let Some(file) = state.open_file() else {
                return true;
            };
            let kind = state.registry.get(&file.graph.nodes[&node].kind);
            self.pickers
                .iter()
                .map(|picker| {
                    kind.and_then(|k| k.props.iter().find(|p| p.name == picker.prop))
                        .map(|p| p.kind)
                })
                .collect()
        };
        for (ix, kind) in kinds.into_iter().enumerate() {
            let Some(kind) = kind else { return false };
            if self.entries(kind, cx).1 != self.pickers[ix].labels {
                return false;
            }
            let value = {
                let state = self.state.read(cx);
                state.open_file().map_or_else(String::new, |file| {
                    prop_text(file, &node, &self.pickers[ix].prop)
                })
            };
            let picker = &mut self.pickers[ix];
            let id_chosen = picker.id.is_some() && is_id(&value);
            // An empty value keeps "Role with ID…" open while the ID is being typed.
            picker.id_chosen = id_chosen || (picker.id_chosen && value.is_empty());
            let selected = if picker.id_chosen {
                Some(picker.values.len())
            } else {
                picker.values.iter().position(|v| *v == value)
            };
            if picker.select.read(cx).selected_index(cx) != selected.map(IndexPath::new) {
                picker.select.update(cx, |select, cx| {
                    select.set_selected_index(selected.map(IndexPath::new), window, cx);
                });
            }
            if let Some(id) = &picker.id
                && id_chosen
                && id.read(cx).value() != value.as_str()
            {
                id.update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
        true
    }

    /// The shown node's command options, as the graph has them.
    fn current_options(&self, cx: &gpui_kit::App) -> Vec<CommandOption> {
        let (Some((_, node)), Some(prop)) = (&self.shown, &self.options_prop) else {
            return Vec::new();
        };
        let state = self.state.read(cx);
        let value = state
            .open_file()
            .and_then(|file| file.graph.nodes.get(node))
            .and_then(|node| node.props.get(prop));
        CommandOption::list(value)
    }

    fn build_options(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let names = type_names();
        self.options = self
            .current_options(cx)
            .into_iter()
            .enumerate()
            .map(|(ix, option)| {
                let mut text = |value: String, placeholder: &str, cx: &mut Context<'_, Self>| {
                    let placeholder = t(placeholder);
                    cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(value)
                            .placeholder(placeholder)
                    })
                };
                let name = text(option.name, "option-name-placeholder", cx);
                let description = text(option.description, "option-description-placeholder", cx);
                let selected = type_index(option.ty).map(IndexPath::new);
                let ty = cx.new(|cx| SelectState::new(names.clone(), selected, window, cx));
                let typing =
                    |input: &Entity<InputState>,
                     set: fn(&mut CommandOption, String),
                     cx: &mut Context<'_, Self>| {
                        cx.subscribe_in(input, window, move |this, input, event, _, cx| match event
                        {
                            InputEvent::Focus => this.fresh = true,
                            InputEvent::Change => {
                                let value = input.read(cx).value().to_string();
                                let undoable = std::mem::take(&mut this.fresh);
                                this.edit_options(undoable, cx, |options| {
                                    set(&mut options[ix], value);
                                });
                            }
                            _ => {}
                        })
                    };
                let name_changes = typing(&name, |option, value| option.name = value, cx);
                let description_changes =
                    typing(&description, |option, value| option.description = value, cx);
                let type_changes = cx.subscribe_in(&ty, window, move |this, _, event, _, cx| {
                    let SelectEvent::Confirm(Some(chosen)) = event else {
                        return;
                    };
                    let Some(ty) = type_names()
                        .iter()
                        .position(|name| name == chosen)
                        .map(|i| CommandOption::TYPES[i])
                    else {
                        return;
                    };
                    this.edit_options(true, cx, |options| options[ix].ty = ty);
                });
                OptionRow {
                    name,
                    description,
                    ty,
                    _changes: [name_changes, description_changes, type_changes],
                }
            })
            .collect();
    }

    /// Rebuilds the rows when options were added or removed, e.g. by undo, and otherwise
    /// brings their text and type up to date.
    fn sync_options(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let options = self.current_options(cx);
        if options.len() != self.options.len() {
            self.build_options(window, cx);
            return;
        }
        for (row, option) in self.options.iter().zip(options) {
            for (input, value) in [
                (&row.name, option.name),
                (&row.description, option.description),
            ] {
                if input.read(cx).value() != value.as_str() {
                    input.update(cx, |input, cx| input.set_value(value, window, cx));
                }
            }
            let selected = type_index(option.ty).map(IndexPath::new);
            if row.ty.read(cx).selected_index(cx) != selected {
                row.ty.update(cx, |select, cx| {
                    select.set_selected_index(selected, window, cx);
                });
            }
        }
    }

    /// Changes the shown node's command options with `change`.
    fn edit_options(
        &mut self,
        undoable: bool,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<CommandOption>),
    ) {
        let (Some((_, node)), Some(prop)) = (self.shown.clone(), self.options_prop.clone()) else {
            return;
        };
        self.state.update(cx, |state, cx| {
            state.edit(undoable, cx, |file, _| {
                let Some(node) = file.graph.nodes.get_mut(&node) else {
                    return;
                };
                let mut options = CommandOption::list(node.props.get(&prop));
                change(&mut options);
                let value = serde_json::to_value(options).expect("options serialize");
                node.props.insert(prop, value);
            });
        });
    }

    /// Adds an option with the first free name, `option1`, `option2`, … in the app's
    /// language. Its description is left for the user; Problems asks for it.
    fn add_option(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_options(true, cx, |options| {
            let base = t("option-default-name");
            // One of the first len + 1 names is always free.
            let name = (1..=options.len() + 1)
                .map(|n| format!("{base}{n}"))
                .find(|name| options.iter().all(|option| &option.name != name))
                .expect("a free name");
            options.push(CommandOption {
                name,
                ty: DataType::Text,
                description: String::new(),
                required: false,
            });
        });
    }

    /// Text field `ix` with its label and, for templates, the {x} variable menu (D-078).
    fn render_text_field(
        &self,
        ix: usize,
        node_id: &str,
        variables: &[String],
        this: &gpui_kit::WeakEntity<Self>,
    ) -> impl IntoElement + use<> {
        let field = &self.fields[ix];
        let name = t_or(&format!("prop-{node_id}-{}", field.prop), &field.prop);
        let picker = (field.templates && !variables.is_empty()).then(|| {
            let (variables, this) = (variables.to_vec(), this.clone());
            Button::new(("insert-variable", ix))
                .ghost()
                .xsmall()
                .label("{x}")
                .dropdown_menu(move |mut menu, _, _| {
                    for variable in &variables {
                        let (this, variable) = (this.clone(), variable.clone());
                        let item =
                            PopupMenuItem::new(variable.clone()).on_click(move |_, window, cx| {
                                let _ = this.update(cx, |this, cx| {
                                    this.insert_variable(ix, &variable, window, cx);
                                });
                            });
                        menu = menu.item(item);
                    }
                    menu
                })
        });
        v_flex()
            .gap(px(4.))
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .child(label(name))
                    .children(picker),
            )
            .child(Input::new(&field.input))
    }

    fn render_options(
        &self,
        label_text: String,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let options = self.current_options(cx);
        v_flex()
            .gap(px(6.))
            .child(label(label_text))
            .children(
                self.options
                    .iter()
                    .zip(options)
                    .enumerate()
                    .map(|(ix, (row, option))| {
                        v_flex()
                            .gap(px(4.))
                            .p(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .child(div().flex_1().child(Input::new(&row.name).small()))
                                    .child(div().w(px(112.)).child(Select::new(&row.ty).small())),
                            )
                            .child(Input::new(&row.description).small())
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        Checkbox::new(("option-required", ix))
                                            .checked(option.required)
                                            .label(t("option-required"))
                                            .on_click(cx.listener(
                                                move |this, checked: &bool, _, cx| {
                                                    let checked = *checked;
                                                    this.edit_options(true, cx, |options| {
                                                        options[ix].required = checked;
                                                    });
                                                },
                                            )),
                                    )
                                    .child(
                                        Button::new(("option-remove", ix))
                                            .ghost()
                                            .xsmall()
                                            .label(t("option-remove"))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.edit_options(true, cx, |options| {
                                                    options.remove(ix);
                                                });
                                            })),
                                    ),
                            )
                    }),
            )
            .child(
                Button::new("option-add")
                    .xsmall()
                    .label(t("option-add"))
                    .on_click(cx.listener(|this, _, _, cx| this.add_option(cx))),
            )
    }

    /// Inserts `variable`, e.g. `{user.name}`, at the cursor of text field `ix` (D-078), as
    /// an undo step of its own.
    fn insert_variable(
        &mut self,
        ix: usize,
        variable: &str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(field) = self.fields.get(ix) else {
            return;
        };
        let input = field.input.clone();
        self.fresh = true;
        input.update(cx, |input, cx| {
            input.insert(variable.to_owned(), window, cx);
        });
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

/// The variables a template in `file` can use, e.g. `{user.name}` or `{option.reason}`.
fn variables(file: &GraphFile, registry: &Registry) -> Vec<String> {
    let mut found = Vec::new();
    for variable in registry.scope(&file.graph) {
        let fields = variable.ty.fields();
        if fields.is_empty() {
            found.push(format!("{{{}}}", variable.name));
        }
        found.extend(
            fields
                .iter()
                .map(|field| format!("{{{}.{field}}}", variable.name)),
        );
    }
    found
}

/// A variable as a picker lists it: "Command author", "Option: target" (D-078).
fn variable_label(name: &str) -> String {
    match name {
        "user" => t("var-user"),
        "channel" => t("var-channel"),
        "server" => t("var-server"),
        _ => match name.strip_prefix("option.") {
            Some(option) => t_with("var-option", &[("name", option)]),
            None => name.to_owned(),
        },
    }
}

/// A fixed list's value in the app's language, e.g. "Ban members".
fn choice_label(choices: rustbot_core::registry::Choices, value: &str) -> String {
    let list = match choices {
        rustbot_core::registry::Choices::Permission => "permission",
        rustbot_core::registry::Choices::Comparison => "comparison",
        rustbot_core::registry::Choices::DeleteMessages => "delete_messages",
    };
    t_or(&format!("choice-{list}-{value}"), value)
}

/// The option types' names, in the order of [`CommandOption::TYPES`].
fn type_names() -> Vec<SharedString> {
    CommandOption::TYPES
        .iter()
        .map(|ty| {
            SharedString::from(t(match ty {
                DataType::Text => "type-text",
                DataType::Number => "type-number",
                DataType::Bool => "type-bool",
                DataType::User => "type-user",
                DataType::Channel => "type-channel",
                DataType::Role => "type-role",
                DataType::Server => "type-server",
            }))
        })
        .collect()
}

fn type_index(ty: DataType) -> Option<usize> {
    CommandOption::TYPES.iter().position(|t| *t == ty)
}

impl Render for Properties {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let body = match (&self.shown, state.open_file()) {
            (Some((_, node)), Some(file)) => {
                let kind = &file.graph.nodes[node].kind;
                let node_id = kind.replace('.', "-");
                let variables = variables(file, &state.registry);
                let this = cx.entity().downgrade();
                v_flex()
                    .gap(px(6.))
                    .child(div().text_sm().child(super::editor::node_name(kind)))
                    .children(self.layout.iter().map(|slot| {
                        match *slot {
                            Slot::Text(ix) => self
                                .render_text_field(ix, &node_id, &variables, &this)
                                .into_any_element(),
                            Slot::Pick(ix) => {
                                let picker = &self.pickers[ix];
                                let name =
                                    t_or(&format!("prop-{node_id}-{}", picker.prop), &picker.prop);
                                v_flex()
                                    .gap(px(4.))
                                    .child(label(name))
                                    .child(Select::new(&picker.select).small())
                                    .children(
                                        picker
                                            .id
                                            .as_ref()
                                            .filter(|_| picker.id_chosen)
                                            .map(|id| Input::new(id).small()),
                                    )
                                    .into_any_element()
                            }
                            Slot::Options => {
                                let prop = self.options_prop.clone().unwrap_or_default();
                                let name = t_or(&format!("prop-{node_id}-{prop}"), &prop);
                                self.render_options(name, cx).into_any_element()
                            }
                        }
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
    use rustbot_core::registry::{CommandOption, DataType};

    use super::Properties;
    use crate::state::AppState;

    /// "+ Add option" adds a text option with a free name; typing its description writes it,
    /// undo takes the option back out, and "Remove" deletes it (D-076).
    #[gpui_kit::test]
    fn the_options_list_edits_the_command(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, _| {
                app.selected_node = Some(NodeId("n1".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        let options = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                CommandOption::list(
                    file.graph.nodes[&NodeId("n1".to_owned())]
                        .props
                        .get("options"),
                )
            })
        };
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("option-add", cx);
        });
        let name = format!("{}1", crate::i18n::t("option-default-name"));
        assert_eq!(
            options(cx),
            vec![CommandOption {
                name: name.clone(),
                ty: DataType::Text,
                description: String::new(),
                required: false,
            }]
        );

        cx.update(|window, cx| {
            let field = properties.read(cx).options[0].description.clone();
            field.update(cx, |input, cx| input.focus(window, cx));
            window.render_frame(cx);
        });
        cx.update(|window, cx| window.input("Who to ban", cx));
        assert_eq!(options(cx)[0].description, "Who to ban");

        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.undo(false, cx));
            window.render_frame(cx);
        });
        assert_eq!(options(cx)[0].description, "");
        let shown = cx.read(|cx| properties.read(cx).options[0].description.read(cx).value());
        assert_eq!(shown.as_str(), "");

        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click(("option-remove", 0usize), cx);
        });
        assert_eq!(options(cx), Vec::new());
        assert_eq!(cx.read(|cx| properties.read(cx).options.len()), 0);
    }

    /// A user field lists the user variables; a role field also offers "Role with ID…",
    /// whose field writes the typed ID; undo brings the list back (D-078, D-080).
    #[gpui_kit::test]
    fn pickers_write_variables_and_ids(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, cx| {
                app.edit(false, cx, |file, registry| {
                    let options = serde_json::json!([
                        { "name": "cel", "type": "user", "description": "Who" },
                        { "name": "rola", "type": "role", "description": "Role" }
                    ]);
                    file.graph
                        .nodes
                        .get_mut(&NodeId("n1".to_owned()))
                        .unwrap()
                        .props
                        .insert("options".to_owned(), options);
                    crate::canvas::add_node(file, registry, "core.has_role", [0., 0.]);
                });
                app.selected_node = Some(NodeId("n3".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        let prop = |cx: &mut gpui_kit::VisualTestContext, name: &str| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                file.graph.nodes[&NodeId("n3".to_owned())].props[name].clone()
            })
        };
        let values = cx.read(|cx| properties.read(cx).pickers[0].values.clone());
        assert_eq!(values, ["user", "option.cel"]);
        let labels = cx.read(|cx| properties.read(cx).pickers[1].labels.clone());
        assert_eq!(labels.len(), 2, "option.rola and Role with ID…");

        cx.update(|window, cx| {
            properties.update(cx, |this, cx| this.pick(0, 1, window, cx));
            properties.update(cx, |this, cx| this.pick(1, 1, window, cx));
            window.render_frame(cx);
        });
        assert_eq!(prop(cx, "user"), "option.cel");
        assert!(cx.read(|cx| properties.read(cx).pickers[1].id_chosen));
        cx.update(|window, cx| window.input("123456789012345678", cx));
        assert_eq!(prop(cx, "role"), "123456789012345678");

        cx.update(|window, cx| {
            properties.update(cx, |this, cx| this.pick(1, 0, window, cx));
            window.render_frame(cx);
        });
        assert_eq!(prop(cx, "role"), "option.rola");
        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.undo(false, cx));
            window.render_frame(cx);
        });
        assert_eq!(prop(cx, "role"), "123456789012345678");
        assert!(cx.read(|cx| properties.read(cx).pickers[1].id_chosen));
    }

    /// {x} inserts the chosen variable at the cursor, as one undo step (D-078).
    #[gpui_kit::test]
    fn a_variable_goes_in_at_the_cursor(cx: &mut TestAppContext) {
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
        cx.update(|window, cx| window.press("home", cx));
        cx.update(|window, cx| {
            properties.update(cx, |this, cx| {
                this.insert_variable(0, "{user.mention} ", window, cx);
            });
        });
        let content = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                file.graph.nodes[&NodeId("n2".to_owned())].props["content"].clone()
            })
        };
        assert_eq!(content(cx), "{user.mention} Pong, {user.name}!");
        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.undo(false, cx));
            window.render_frame(cx);
        });
        assert_eq!(content(cx), "Pong, {user.name}!");
    }

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
