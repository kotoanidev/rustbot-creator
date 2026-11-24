//! The side bar's form (D-066): the selected node's fields, with the settings of the memory
//! variable it uses (D-109), or the bot's settings when no node is selected: its token,
//! kept in the system keyring (D-020), and the test server (D-055, D-065).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::link::Link;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IndexPath, Sizable as _, h_flex, v_flex,
};
use gpui_kit::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, Window, div, px,
};
use rustbot_core::registry::{
    ButtonStyle, CodeInput, CommandOption, DataType, MenuOption, PropKind, valid_memory_name,
};
use rustbot_core::{GraphFile, MemoryScope, MemoryType, NodeId, Registry};

use crate::i18n::{t, t_or, t_with};
use crate::state::AppState;

pub(crate) struct Properties {
    focus: FocusHandle,
    state: Entity<AppState>,
    /// The node the fields belong to: `(graph index, node id)`.
    shown: Option<(usize, NodeId)>,
    fields: Vec<Field>,
    pickers: Vec<Picker>,
    /// The switches' props, drawn from the graph.
    checks: Vec<String>,
    /// Whether the "Embed" section is open (D-087).
    embed_open: bool,
    /// Whether the "Buttons and menu" section is open (D-098).
    clicks_open: bool,
    /// The shown message's buttons and menu options (D-098, D-099).
    buttons: Vec<ItemRow>,
    menu: Vec<ItemRow>,
    /// The shown form's text fields (D-112).
    form_fields: Vec<FieldRow>,
    /// The shown "Custom code" node's inputs (D-176).
    code_inputs: Vec<CodeRow>,
    /// The form's fields in the node type's order.
    layout: Vec<Slot>,
    /// The prop holding the shown node's command options, if it has one (D-076).
    options_prop: Option<String>,
    options: Vec<OptionRow>,
    /// The next change starts a new undo step: set when a field gains the focus.
    fresh: bool,
    bot: Option<BotForm>,
    /// The settings of the memory variable the shown node uses (D-109).
    memory: Option<MemoryForm>,
    /// The name field of "New variable…" (D-109).
    new_memory: Option<NewMemory>,
}

struct NewMemory {
    input: Entity<InputState>,
    error: Option<String>,
    _changes: Subscription,
}

/// A memory variable's settings (D-107, D-109), shared by every node that uses it.
struct MemoryForm {
    ix: usize,
    ty: Entity<SelectState<Vec<SharedString>>>,
    scope: Entity<SelectState<Vec<SharedString>>>,
    initial: Entity<InputState>,
    /// The starting value typed for a number is not one, so it is not saved.
    bad_initial: bool,
    _changes: [Subscription; 3],
}

/// The memory types, in the order the form lists them.
const MEMORY_TYPES: [MemoryType; 2] = [MemoryType::Number, MemoryType::Text];

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
    Check(usize),
    Options,
    Buttons,
    Menu,
    FormFields,
    CodeInputs,
}

/// The editable parts of a "Custom code" input (D-176).
struct CodeRow {
    name: Entity<InputState>,
    ty: Entity<SelectState<Vec<SharedString>>>,
    value: Entity<InputState>,
    _changes: Vec<Subscription>,
}

/// The typed parts of a form's text field (D-112); "long" and "required" are drawn from
/// the graph.
struct FieldRow {
    name: Entity<InputState>,
    label: Entity<InputState>,
    placeholder: Entity<InputState>,
    min: Entity<InputState>,
    max: Entity<InputState>,
    _changes: Vec<Subscription>,
}

/// A form field's typed keys, in the order of [`FieldRow`]'s inputs.
const FIELD_KEYS: [&str; 5] = ["name", "label", "placeholder", "min", "max"];

/// The fields of the "Buttons and menu" section, in the node type's order (D-097–D-100).
const CLICK_PROPS: [&str; 6] = [
    "buttons",
    "menu",
    "menu_placeholder",
    "wait",
    "only_author",
    "not_yours",
];

/// The editable parts of one button or menu option (D-098, D-099). A button also has a
/// colour and, for a link, an address.
struct ItemRow {
    label: Entity<InputState>,
    style: Option<Entity<SelectState<Vec<SharedString>>>>,
    url: Option<Entity<InputState>>,
    _changes: Vec<Subscription>,
}

/// A field picked from a list: a variable of a type (D-078), or one value of a fixed list.
struct Picker {
    prop: String,
    kind: PropKind,
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
    license: Entity<SelectState<Vec<SharedString>>>,
    _changes: [Subscription; 3],
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
            checks: Vec::new(),
            embed_open: false,
            clicks_open: false,
            buttons: Vec::new(),
            menu: Vec::new(),
            form_fields: Vec::new(),
            code_inputs: Vec::new(),
            layout: Vec::new(),
            options_prop: None,
            options: Vec::new(),
            fresh: true,
            bot: None,
            memory: None,
            new_memory: None,
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
            self.embed_open = false;
            self.clicks_open = false;
            self.new_memory = None;
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
            self.sync_items(window, cx);
        }
        if self.shown.is_none() && self.bot.is_none() {
            self.bot = Some(self.build_bot_form(window, cx));
        }
        let state = self.state.read(cx);
        let memory = self.shown.as_ref().and_then(|(_, node)| {
            let props = &state.open_file()?.graph.nodes.get(node)?.props;
            let name = props.get("variable")?.as_str()?;
            let memory = &state.project.manifest.memory;
            memory.iter().position(|var| var.name == name)
        });
        if memory != self.memory.as_ref().map(|form| form.ix) {
            self.memory = memory.map(|ix| self.build_memory_form(ix, window, cx));
        }
        cx.notify();
    }

    fn build_memory_form(
        &self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> MemoryForm {
        let var = self.state.read(cx).project.manifest.memory[ix].clone();
        let types: Vec<SharedString> = MEMORY_TYPES.map(|ty| memory_type_name(ty).into()).into();
        let scopes: Vec<SharedString> = MemoryScope::ALL.map(|s| scope_name(s).into()).into();
        let at = |found: Option<usize>| found.map(IndexPath::new);
        let ty_at = at(MEMORY_TYPES.iter().position(|ty| *ty == var.ty));
        let scope_at = at(MemoryScope::ALL.iter().position(|s| *s == var.scope));
        let ty = cx.new(|cx| SelectState::new(types, ty_at, window, cx));
        let scope = cx.new(|cx| SelectState::new(scopes, scope_at, window, cx));
        let initial = cx.new(|cx| InputState::new(window, cx).default_value(var.initial));
        let changes = [
            cx.subscribe_in(&ty, window, move |this, _, event, window, cx| {
                let SelectEvent::Confirm(Some(label)) = event else {
                    return;
                };
                let Some(ty) = MEMORY_TYPES
                    .into_iter()
                    .find(|ty| memory_type_name(*ty) == label.as_ref())
                else {
                    return;
                };
                // A starting value that is not a number goes when the variable becomes one.
                let clear = ty == MemoryType::Number
                    && this
                        .memory
                        .as_ref()
                        .is_some_and(|form| !is_number_or_empty(&form.initial.read(cx).value()));
                if clear && let Some(form) = &this.memory {
                    form.initial
                        .update(cx, |input, cx| input.set_value("", window, cx));
                }
                this.state.update(cx, |state, cx| {
                    state.change_memory(
                        ix,
                        |var| {
                            var.ty = ty;
                            if clear {
                                var.initial.clear();
                            }
                        },
                        cx,
                    );
                });
            }),
            cx.subscribe_in(&scope, window, move |this, _, event, _, cx| {
                let SelectEvent::Confirm(Some(label)) = event else {
                    return;
                };
                if let Some(scope) = MemoryScope::ALL
                    .into_iter()
                    .find(|scope| scope_name(*scope) == label.as_ref())
                {
                    this.state.update(cx, |state, cx| {
                        state.change_memory(ix, |var| var.scope = scope, cx);
                    });
                }
            }),
            cx.subscribe_in(&initial, window, move |this, input, event, _, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let value = input.read(cx).value().to_string();
                let number = this
                    .state
                    .read(cx)
                    .project
                    .manifest
                    .memory
                    .get(ix)
                    .is_some_and(|var| var.ty == MemoryType::Number);
                let bad = number && !is_number_or_empty(&value);
                if let Some(form) = &mut this.memory {
                    form.bad_initial = bad;
                }
                if !bad {
                    this.state.update(cx, |state, cx| {
                        state.change_memory(ix, |var| var.initial = value, cx);
                    });
                }
                cx.notify();
            }),
        ];
        MemoryForm {
            ix,
            ty,
            scope,
            initial,
            bad_initial: false,
            _changes: changes,
        }
    }

    /// "New variable…" was picked: asks for the name.
    fn start_new_memory(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder(t("new-memory-placeholder")));
        input.update(cx, |input, cx| input.focus(window, cx));
        let changes = cx.subscribe_in(&input, window, |this, input, event, _, cx| match event {
            InputEvent::PressEnter { .. } => {
                let name = input.read(cx).value().trim().to_owned();
                this.create_memory(&name, cx);
            }
            // Leaving an empty field gives up.
            InputEvent::Blur if input.read(cx).value().trim().is_empty() => {
                this.new_memory = None;
                cx.notify();
            }
            _ => {}
        });
        self.new_memory = Some(NewMemory {
            input,
            error: None,
            _changes: changes,
        });
        cx.notify();
    }

    /// Makes memory variable `name` and picks it for the shown node (D-109). It holds text
    /// when "Set" already has text typed, else a number.
    fn create_memory(&mut self, name: &str, cx: &mut Context<'_, Self>) {
        let memory = &self.state.read(cx).project.manifest.memory;
        let error = if !valid_memory_name(name) {
            Some(t("new-memory-invalid"))
        } else if memory.iter().any(|var| var.name == name) {
            Some(t("new-memory-taken"))
        } else {
            None
        };
        if let Some(error) = error {
            if let Some(new) = &mut self.new_memory {
                new.error = Some(error);
            }
            cx.notify();
            return;
        }
        let value = self.text("value", cx);
        let value = value.trim();
        let ty = if value.is_empty() || value.contains('{') || value.parse::<f64>().is_ok() {
            MemoryType::Number
        } else {
            MemoryType::Text
        };
        self.new_memory = None;
        self.state
            .update(cx, |state, cx| state.add_memory(name, ty, cx));
        self.fresh = true;
        self.write("variable", name.to_owned(), cx);
    }

    /// The "Variable" list, the name field of "New variable…", and the picked variable's
    /// settings (D-109).
    fn render_memory_picker(
        &self,
        ix: usize,
        node_id: &str,
        cx: &Context<'_, Self>,
    ) -> gpui_kit::AnyElement {
        let theme = cx.theme();
        let new = self.new_memory.as_ref().map(|new| {
            v_flex()
                .gap(px(2.))
                .child(Input::new(&new.input).small())
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(t("memory-new-hint")),
                )
                .children(
                    new.error
                        .clone()
                        .map(|error| div().text_xs().text_color(theme.danger).child(error)),
                )
        });
        v_flex()
            .gap(px(6.))
            .child(self.render_picker(ix, node_id))
            .children(new)
            .children(
                self.memory
                    .as_ref()
                    .filter(|_| self.new_memory.is_none())
                    .map(|form| self.render_memory_form(form, cx)),
            )
            .into_any_element()
    }

    fn render_memory_form(
        &self,
        form: &MemoryForm,
        cx: &Context<'_, Self>,
    ) -> gpui_kit::AnyElement {
        let name = self
            .state
            .read(cx)
            .project
            .manifest
            .memory
            .get(form.ix)
            .map_or_else(String::new, |var| var.name.clone());
        let theme = cx.theme();
        let note = |text: String, error: bool| {
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
            .pl(px(8.))
            .border_l_2()
            .border_color(theme.border)
            .child(note(t("memory-shared-hint"), false))
            .child(label(t("memory-type")))
            .child(Select::new(&form.ty).small())
            .child(label(t("memory-scope")))
            .child(Select::new(&form.scope).small())
            .child(label(t("memory-initial")))
            .child(Input::new(&form.initial).small())
            .children(
                form.bad_initial
                    .then(|| note(t("memory-initial-not-number"), true)),
            )
            .child(note(t_with("memory-read-hint", &[("name", &name)]), false))
            .into_any_element()
    }

    fn build_fields(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.fields.clear();
        self.pickers.clear();
        self.checks.clear();
        self.layout.clear();
        self.options_prop = None;
        self.options.clear();
        self.buttons.clear();
        self.menu.clear();
        self.form_fields.clear();
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
            // A filled-in embed shows its section (D-087).
            self.embed_open |= prop.starts_with("embed_") && !value.is_empty();
            self.clicks_open |= CLICK_PROPS.contains(&prop.as_str())
                && !matches!(value.as_str(), "" | "[]" | "false");
            match kind {
                PropKind::Bool => {
                    self.layout.push(Slot::Check(self.checks.len()));
                    self.checks.push(prop);
                }
                PropKind::Text { .. } | PropKind::Number { .. } | PropKind::Color => {
                    // A number or colour may come from a variable too (D-084).
                    let templates = kind != PropKind::Text { templates: false };
                    self.layout.push(Slot::Text(self.fields.len()));
                    let field = Self::text_field(prop, templates, value, window, cx);
                    self.fields.push(field);
                }
                PropKind::Variable(_) | PropKind::Choice(_) | PropKind::Memory => {
                    self.layout.push(Slot::Pick(self.pickers.len()));
                    let picker = self.picker(prop, kind, &value, window, cx);
                    self.pickers.push(picker);
                }
                PropKind::Options => {
                    self.layout.push(Slot::Options);
                    self.options_prop = Some(prop);
                }
                PropKind::Buttons => self.layout.push(Slot::Buttons),
                PropKind::MenuOptions => self.layout.push(Slot::Menu),
                PropKind::FormFields => self.layout.push(Slot::FormFields),
                PropKind::CodeInputs => self.layout.push(Slot::CodeInputs),
            }
        }
        self.build_options(window, cx);
        self.buttons = self.item_rows("buttons", window, cx);
        self.menu = self.item_rows("menu", window, cx);
        self.form_fields = self.field_rows(window, cx);
        self.code_inputs = self.code_rows(window, cx);
    }

    /// The shown node's buttons or menu options, as the graph has them.
    fn current_items(&self, prop: &str, cx: &gpui_kit::App) -> Vec<serde_json::Value> {
        let Some((_, node)) = &self.shown else {
            return Vec::new();
        };
        let state = self.state.read(cx);
        match state
            .open_file()
            .and_then(|file| file.graph.nodes.get(node))
            .and_then(|node| node.props.get(prop))
        {
            Some(serde_json::Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        }
    }

    fn item_rows(
        &self,
        prop: &'static str,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<ItemRow> {
        self.current_items(prop, cx)
            .into_iter()
            .enumerate()
            .map(|(ix, item)| {
                let text = |key: &str| {
                    item.get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                };
                let label = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(text("label"))
                        .placeholder(t("item-label-placeholder"))
                });
                let mut changes = vec![item_typing(prop, ix, "label", &label, window, cx)];
                let (style, url) = if prop == "buttons" {
                    let selected = style_index(&text("style")).map(IndexPath::new);
                    let style = cx.new(|cx| SelectState::new(style_names(), selected, window, cx));
                    changes.push(
                        cx.subscribe_in(&style, window, move |this, _, event, _, cx| {
                            let SelectEvent::Confirm(Some(chosen)) = event else {
                                return;
                            };
                            let Some(style) = style_names().iter().position(|name| name == chosen)
                            else {
                                return;
                            };
                            this.edit_items("buttons", true, cx, |items| {
                                set_key(items, ix, "style", style_value(ButtonStyle::ALL[style]));
                            });
                        }),
                    );
                    let url = cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(text("url"))
                            .placeholder("https://…")
                    });
                    changes.push(item_typing(prop, ix, "url", &url, window, cx));
                    (Some(style), Some(url))
                } else {
                    (None, None)
                };
                ItemRow {
                    label,
                    style,
                    url,
                    _changes: changes,
                }
            })
            .collect()
    }

    /// The "wait" field of a message (D-097) or a form (D-111), with what it means.
    fn render_wait(
        &self,
        ix: usize,
        node_id: &str,
        variables: &[String],
        this: &gpui_kit::WeakEntity<Self>,
        cx: &Context<'_, Self>,
    ) -> gpui_kit::AnyElement {
        let hint = if node_id == "core-show_form" {
            "form-wait-hint"
        } else {
            "wait-hint"
        };
        v_flex()
            .gap(px(2.))
            .child(self.render_text_field(ix, node_id, variables, this))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(hint)),
            )
            .into_any_element()
    }

    /// The rows of the shown form's text fields (D-112).
    fn field_rows(&self, window: &mut Window, cx: &mut Context<'_, Self>) -> Vec<FieldRow> {
        self.current_items("fields", cx)
            .into_iter()
            .enumerate()
            .map(|(ix, item)| {
                let mut changes = Vec::new();
                let inputs = FIELD_KEYS.map(|key| {
                    let value = item
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let input = cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(value)
                            .placeholder(t(&format!("form-field-{key}")))
                    });
                    changes.push(item_typing("fields", ix, key, &input, window, cx));
                    input
                });
                let [name, label, placeholder, min, max] = inputs;
                FieldRow {
                    name,
                    label,
                    placeholder,
                    min,
                    max,
                    _changes: changes,
                }
            })
            .collect()
    }

    /// The rows of the shown "Custom code" node's inputs (D-176).
    fn code_rows(&self, window: &mut Window, cx: &mut Context<'_, Self>) -> Vec<CodeRow> {
        self.current_items("inputs", cx)
            .into_iter()
            .enumerate()
            .map(|(ix, item)| {
                let text = |key: &str| {
                    item.get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                };
                let mut input = |key: &str, cx: &mut Context<'_, Self>| {
                    let (value, hint) = (text(key), t(&format!("code-input-{key}")));
                    cx.new(|cx| {
                        InputState::new(window, cx)
                            .default_value(value)
                            .placeholder(hint)
                    })
                };
                let name = input("name", cx);
                let value = input("value", cx);
                let selected = code_type_index(&text("type")).map(IndexPath::new);
                let ty = cx.new(|cx| SelectState::new(code_type_names(), selected, window, cx));
                let mut changes = vec![
                    item_typing("inputs", ix, "name", &name, window, cx),
                    item_typing("inputs", ix, "value", &value, window, cx),
                ];
                changes.push(cx.subscribe_in(&ty, window, move |this, _, event, _, cx| {
                    let SelectEvent::Confirm(Some(chosen)) = event else {
                        return;
                    };
                    let Some(ty) = code_type_names().iter().position(|name| name == chosen) else {
                        return;
                    };
                    let ty = data_type_value(CodeInput::TYPES[ty]);
                    this.edit_items("inputs", true, cx, |items| set_key(items, ix, "type", ty));
                }));
                CodeRow {
                    name,
                    ty,
                    value,
                    _changes: changes,
                }
            })
            .collect()
    }

    /// Adds a text input with a free name, `input2`.
    fn add_code_input(&mut self, cx: &mut Context<'_, Self>) {
        self.edit_items("inputs", true, cx, |items| {
            let taken = |name: &str| {
                items
                    .iter()
                    .any(|item| item.get("name") == Some(&name.into()))
            };
            let next = (1..=items.len() + 1)
                .find(|n| !taken(&format!("input{n}")))
                .unwrap_or(1);
            items.push(serde_json::json!({
                "name": format!("input{next}"), "type": "text", "value": "",
            }));
        });
    }

    /// "Create function": the shown node's function, added to `src/lib.rs` (D-176).
    fn create_function(&self, cx: &mut Context<'_, Self>) {
        let Some((_, node)) = self.shown.clone() else {
            return;
        };
        self.state.update(cx, |state, cx| {
            let stub = state.open_file().and_then(|file| {
                let function = file.graph.nodes.get(&node)?.props.get("function")?;
                let function = function.as_str()?.to_owned();
                Some((function, rustbot_codegen::code_stub(file, &node)?))
            });
            let Some((function, stub)) = stub else {
                state.log_error(t("code-create-invalid"));
                cx.notify();
                return;
            };
            let root = state.project.root.clone();
            let added = crate::code::add_function(&root, &function, &stub);
            if added.is_ok() {
                // The function, where it can be written (D-177).
                let path = std::path::PathBuf::from("src/lib.rs");
                let code = crate::code::read(&root, &path).unwrap_or_default();
                let line = crate::code::function_line(&code, &function);
                cx.emit(crate::state::OpenCode { path, line });
            }
            match added {
                Ok(true) => state.log(t_with("code-created", &[("function", &function)])),
                Ok(false) => state.log(t_with("code-exists", &[("function", &function)])),
                Err(e) => {
                    state.log_error(t_with("code-create-failed", &[("error", &e.to_string())]));
                }
            }
            cx.notify();
        });
    }

    fn render_code_inputs(
        &self,
        label_text: String,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let hint = |id: &str| {
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(t(id))
        };
        v_flex()
            .gap(px(6.))
            .child(label(label_text))
            .children(self.code_inputs.iter().enumerate().map(|(ix, row)| {
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
                            .child(div().w(px(120.)).child(Select::new(&row.ty).small()))
                            .child(
                                Button::new(("code-input-remove", ix))
                                    .ghost()
                                    .xsmall()
                                    .label(t("option-remove"))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.edit_items("inputs", true, cx, |items| {
                                            if ix < items.len() {
                                                items.remove(ix);
                                            }
                                        });
                                    })),
                            ),
                    )
                    .child(Input::new(&row.value).small())
            }))
            .child(
                Button::new("code-input-add")
                    .xsmall()
                    .label(t("code-input-add"))
                    .on_click(cx.listener(|this, _, _, cx| this.add_code_input(cx))),
            )
            .child(hint("code-inputs-hint"))
            .child(
                Button::new("code-create")
                    .primary()
                    .small()
                    .label(t("code-create"))
                    .on_click(cx.listener(|this, _, _, cx| this.create_function(cx))),
            )
            .child(hint("code-create-hint"))
    }

    /// Adds a text field with a free name, `field3`, and a label such as "Field 3" in the
    /// app's language (D-112).
    fn add_form_field(&mut self, cx: &mut Context<'_, Self>) {
        let base = t("form-field-default-label");
        self.edit_items("fields", true, cx, |items| {
            let taken = |name: &str| {
                items
                    .iter()
                    .any(|item| item.get("name") == Some(&name.into()))
            };
            let next = (1..=items.len() + 1)
                .find(|n| !taken(&format!("field{n}")))
                .unwrap_or(1);
            items.push(serde_json::json!({
                "name": format!("field{next}"),
                "label": format!("{base} {next}"),
                "required": true,
            }));
        });
    }

    fn render_form_fields(
        &self,
        label_text: String,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let items = self.current_items("fields", cx);
        v_flex()
            .gap(px(6.))
            .child(label(label_text))
            .children(
                self.form_fields
                    .iter()
                    .zip(&items)
                    .enumerate()
                    .map(|(ix, (row, item))| {
                        let flag =
                            |key: &str| item.get(key) == Some(&serde_json::Value::Bool(true));
                        let check = |key: &'static str, id: &'static str| {
                            Checkbox::new((id, ix))
                                .checked(flag(key))
                                .label(t(&format!("form-field-{key}")))
                                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                                    let checked = *checked;
                                    this.edit_items("fields", true, cx, |items| {
                                        set_key(items, ix, key, checked);
                                    });
                                }))
                        };
                        v_flex()
                            .gap(px(4.))
                            .p(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .child(div().w(px(96.)).child(Input::new(&row.name).small()))
                                    .child(div().flex_1().child(Input::new(&row.label).small()))
                                    .child(
                                        Button::new(("field-remove", ix))
                                            .ghost()
                                            .xsmall()
                                            .label(t("option-remove"))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.edit_items("fields", true, cx, |items| {
                                                    if ix < items.len() {
                                                        items.remove(ix);
                                                    }
                                                });
                                            })),
                                    ),
                            )
                            .child(Input::new(&row.placeholder).small())
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .child(div().flex_1().child(Input::new(&row.min).small()))
                                    .child(div().flex_1().child(Input::new(&row.max).small())),
                            )
                            .child(
                                h_flex()
                                    .gap(px(12.))
                                    .child(check("long", "field-long"))
                                    .child(check("required", "field-required")),
                            )
                    }),
            )
            .child(
                Button::new("field-add")
                    .xsmall()
                    .label(t("form-field-add"))
                    .disabled(items.len() >= 5)
                    .on_click(cx.listener(|this, _, _, cx| this.add_form_field(cx))),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t("form-fields-hint")),
            )
    }

    /// Rebuilds the buttons' and options' rows when some were added or removed, and
    /// otherwise brings their text and colour up to date.
    fn sync_items(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let fields = self.current_items("fields", cx);
        if fields.len() == self.form_fields.len() {
            for (row, item) in self.form_fields.iter().zip(&fields) {
                let inputs = [&row.name, &row.label, &row.placeholder, &row.min, &row.max];
                for (input, key) in inputs.into_iter().zip(FIELD_KEYS) {
                    let value = item
                        .get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    if input.read(cx).value() != value.as_str() {
                        input.update(cx, |input, cx| input.set_value(value, window, cx));
                    }
                }
            }
        } else {
            self.form_fields = self.field_rows(window, cx);
        }
        let inputs = self.current_items("inputs", cx);
        if inputs.len() == self.code_inputs.len() {
            for (row, item) in self.code_inputs.iter().zip(&inputs) {
                let text = |key: &str| {
                    item.get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                };
                for (input, key) in [(&row.name, "name"), (&row.value, "value")] {
                    let value = text(key);
                    if input.read(cx).value() != value.as_str() {
                        input.update(cx, |input, cx| input.set_value(value, window, cx));
                    }
                }
                let selected = code_type_index(&text("type")).map(IndexPath::new);
                if row.ty.read(cx).selected_index(cx) != selected {
                    row.ty.update(cx, |select, cx| {
                        select.set_selected_index(selected, window, cx);
                    });
                }
            }
        } else {
            self.code_inputs = self.code_rows(window, cx);
        }
        for prop in ["buttons", "menu"] {
            let items = self.current_items(prop, cx);
            let rows = if prop == "buttons" {
                &self.buttons
            } else {
                &self.menu
            };
            if items.len() != rows.len() {
                let rows = self.item_rows(prop, window, cx);
                if prop == "buttons" {
                    self.buttons = rows;
                } else {
                    self.menu = rows;
                }
                continue;
            }
            for (row, item) in rows.iter().zip(&items) {
                let text = |key: &str| {
                    item.get(key)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                };
                for (input, key) in [(Some(&row.label), "label"), (row.url.as_ref(), "url")] {
                    let Some(input) = input else { continue };
                    let value = text(key);
                    if input.read(cx).value() != value.as_str() {
                        input.update(cx, |input, cx| input.set_value(value, window, cx));
                    }
                }
                if let Some(style) = &row.style {
                    let selected = style_index(&text("style")).map(IndexPath::new);
                    if style.read(cx).selected_index(cx) != selected {
                        style.update(cx, |select, cx| {
                            select.set_selected_index(selected, window, cx);
                        });
                    }
                }
            }
        }
    }

    /// Changes the shown node's buttons or menu options with `change`. The wires from the
    /// outputs of removed ones go too.
    fn edit_items(
        &mut self,
        prop: &str,
        undoable: bool,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut Vec<serde_json::Value>),
    ) {
        let Some((_, node)) = self.shown.clone() else {
            return;
        };
        let prop = prop.to_owned();
        self.state.update(cx, |state, cx| {
            state.edit(undoable, cx, |file, registry| {
                let Some(shown) = file.graph.nodes.get_mut(&node) else {
                    return;
                };
                let mut items = match shown.props.get(&prop) {
                    Some(serde_json::Value::Array(items)) => items.clone(),
                    _ => Vec::new(),
                };
                change(&mut items);
                shown.props.insert(prop, serde_json::Value::Array(items));
                prune_wires(file, registry, &node);
            });
        });
    }

    /// Adds a button or menu option with the next free id, `b3` or `o3`, and a label such
    /// as "Button 3" in the app's language.
    fn add_item(&mut self, prop: &'static str, cx: &mut Context<'_, Self>) {
        let (prefix, base) = if prop == "buttons" {
            ('b', t("button-default"))
        } else {
            ('o', t("menu-option-default"))
        };
        self.edit_items(prop, true, cx, |items| {
            let next = items
                .iter()
                .filter_map(|item| {
                    item.get("id")?
                        .as_str()?
                        .strip_prefix(prefix)?
                        .parse::<u32>()
                        .ok()
                })
                .max()
                .unwrap_or(0)
                + 1;
            let mut item = serde_json::json!({
                "id": format!("{prefix}{next}"),
                "label": format!("{base} {next}"),
            });
            if prop == "buttons" {
                item["style"] = style_value(ButtonStyle::Primary).into();
            }
            items.push(item);
        });
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
                let node = self.shown.as_ref().map(|(_, node)| node);
                state
                    .open_file()
                    .zip(node)
                    .map_or_else(Vec::new, |(file, node)| {
                        // After a click that always works, only the click's variables (D-097).
                        state
                            .registry
                            .scope_at(&file.graph, node)
                            .into_iter()
                            .filter(|var| var.ty == ty)
                            .map(|var| var.name)
                            .collect()
                    })
            }
            PropKind::Choice(choices) => choices.values().iter().map(|&v| v.to_owned()).collect(),
            PropKind::Memory => {
                let memory = self.state.read(cx).registry.memory();
                memory.iter().map(|var| var.name.clone()).collect()
            }
            PropKind::Text { .. }
            | PropKind::Options
            | PropKind::Number { .. }
            | PropKind::Bool
            | PropKind::Color
            | PropKind::Buttons
            | PropKind::MenuOptions
            | PropKind::FormFields
            | PropKind::CodeInputs => Vec::new(),
        };
        let mut labels: Vec<SharedString> = values
            .iter()
            .map(|value| match kind {
                PropKind::Variable(_) => variable_label(value).into(),
                PropKind::Choice(choices) => choice_label(choices, value).into(),
                PropKind::Text { .. }
                | PropKind::Options
                | PropKind::Number { .. }
                | PropKind::Bool
                | PropKind::Color
                | PropKind::Buttons
                | PropKind::MenuOptions
                | PropKind::FormFields
                | PropKind::CodeInputs
                | PropKind::Memory => value.clone().into(),
            })
            .collect();
        match kind {
            PropKind::Variable(DataType::Role) => labels.push(t("var-role-id").into()),
            PropKind::Variable(DataType::Channel) => labels.push(t("var-channel-id").into()),
            PropKind::Memory => labels.push(t("memory-new").into()),
            _ => {}
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
        // Roles and channels may be typed as IDs (D-080, D-088).
        let takes_id = matches!(kind, PropKind::Variable(DataType::Role | DataType::Channel));
        // Anything typed that is not a listed variable is an ID, whole or still being typed.
        let id_chosen = takes_id && !value.is_empty() && !values.iter().any(|v| v == value);
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
                    .placeholder(t(if kind == PropKind::Variable(DataType::Role) {
                        "role-id-placeholder"
                    } else {
                        "channel-id-placeholder"
                    }))
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
            kind,
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
        if picker.kind == PropKind::Memory && entry >= picker.values.len() {
            self.start_new_memory(window, cx);
            return;
        }
        self.new_memory = None;
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
        let creating = self.new_memory.is_some();
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
            // A typed value that is no listed variable is an ID, whole or still being typed;
            // an empty one keeps "Role with ID…" open if it was chosen.
            let id_chosen =
                picker.id.is_some() && !value.is_empty() && !picker.values.contains(&value);
            picker.id_chosen = id_chosen || (picker.id_chosen && value.is_empty());
            let selected = if picker.id_chosen || (picker.kind == PropKind::Memory && creating) {
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

    /// The form's rows in the node type's order, with the `embed_…` fields under a header
    /// that opens and closes them (D-087).
    fn rows(
        &self,
        file: &GraphFile,
        node: &NodeId,
        node_id: &str,
        variables: &[String],
        this: &gpui_kit::WeakEntity<Self>,
        cx: &Context<'_, Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        let mut rows = Vec::new();
        let mut header = false;
        let mut clicks_header = false;
        let props = &file.graph.nodes[node].props;
        for slot in &self.layout {
            let prop = match *slot {
                Slot::Text(ix) => &self.fields[ix].prop,
                Slot::Pick(ix) => &self.pickers[ix].prop,
                Slot::Check(ix) => &self.checks[ix],
                Slot::Options => "",
                Slot::Buttons => "buttons",
                Slot::Menu => "menu",
                Slot::FormFields => "fields",
                Slot::CodeInputs => "inputs",
            };
            // A form's "wait" is its own, not a click's (D-111).
            if CLICK_PROPS.contains(&prop) && node_id != "core-show_form" {
                if !clicks_header {
                    clicks_header = true;
                    rows.push(section("clicks-section", self.clicks_open, cx, |this| {
                        this.clicks_open = !this.clicks_open;
                    }));
                }
                let hidden = match prop {
                    "menu_placeholder" => MenuOption::list(props.get("menu")).is_empty(),
                    "not_yours" => props.get("only_author") != Some(&serde_json::Value::Bool(true)),
                    _ => false,
                };
                if !self.clicks_open || hidden {
                    continue;
                }
            }
            if prop.starts_with("embed_") {
                if !header {
                    header = true;
                    rows.push(section("embed-section", self.embed_open, cx, |this| {
                        this.embed_open = !this.embed_open;
                    }));
                }
                if !self.embed_open {
                    continue;
                }
            }
            rows.push(match *slot {
                Slot::Text(ix) if self.fields[ix].prop == "wait" => {
                    self.render_wait(ix, node_id, variables, this, cx)
                }
                Slot::FormFields => {
                    let name = t_or(&format!("prop-{node_id}-fields"), "fields");
                    self.render_form_fields(name, cx).into_any_element()
                }
                Slot::CodeInputs => {
                    let name = t_or(&format!("prop-{node_id}-inputs"), "inputs");
                    self.render_code_inputs(name, cx).into_any_element()
                }
                Slot::Text(ix) => self
                    .render_text_field(ix, node_id, variables, this)
                    .into_any_element(),
                Slot::Buttons | Slot::Menu => {
                    let prop = if matches!(*slot, Slot::Buttons) {
                        "buttons"
                    } else {
                        "menu"
                    };
                    let name = t_or(&format!("prop-{node_id}-{prop}"), prop);
                    self.render_items(prop, name, cx).into_any_element()
                }
                Slot::Pick(ix) if self.pickers[ix].kind == PropKind::Memory => {
                    self.render_memory_picker(ix, node_id, cx)
                }
                Slot::Pick(ix) => self.render_picker(ix, node_id).into_any_element(),
                Slot::Check(ix) => {
                    let prop = self.checks[ix].clone();
                    let checked = file.graph.nodes[node].props.get(&prop)
                        == Some(&serde_json::Value::Bool(true));
                    let name = t_or(&format!("prop-{node_id}-{prop}"), &prop);
                    Checkbox::new(("prop-check", ix))
                        .checked(checked)
                        .label(name)
                        .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                            this.fresh = true;
                            this.write(&prop, *checked, cx);
                            // Others get a private answer, in the app's language (D-100).
                            if prop == "only_author"
                                && *checked
                                && this.text("not_yours", cx).is_empty()
                            {
                                this.write("not_yours", t("not-yours-default"), cx);
                            }
                        }))
                        .into_any_element()
                }
                Slot::Options => {
                    let prop = self.options_prop.clone().unwrap_or_default();
                    let name = t_or(&format!("prop-{node_id}-{prop}"), &prop);
                    self.render_options(name, cx).into_any_element()
                }
            });
        }
        rows
    }

    /// Picker `ix` with its label and, after "Role with ID…", the ID's field (D-080).
    fn render_picker(&self, ix: usize, node_id: &str) -> impl IntoElement + use<> {
        let picker = &self.pickers[ix];
        let name = t_or(&format!("prop-{node_id}-{}", picker.prop), &picker.prop);
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
    fn write(
        &mut self,
        prop: &str,
        value: impl Into<serde_json::Value>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((_, node)) = self.shown.clone() else {
            return;
        };
        let undoable = std::mem::take(&mut self.fresh);
        self.state.update(cx, |state, cx| {
            state.edit(undoable, cx, |file, registry| {
                if let Some(shown) = file.graph.nodes.get_mut(&node) {
                    shown.props.insert(prop.to_owned(), value.into());
                }
                // Clearing "wait" removes "timeout" (D-097).
                prune_wires(file, registry, &node);
            });
        });
    }

    /// The shown node's field `prop` as text.
    fn text(&self, prop: &str, cx: &gpui_kit::App) -> String {
        let state = self.state.read(cx);
        match (&self.shown, state.open_file()) {
            (Some((_, node)), Some(file)) => prop_text(file, node, prop),
            _ => String::new(),
        }
    }

    /// The buttons' or menu options' rows, with "Add" (D-098, D-099).
    fn render_items(
        &self,
        prop: &'static str,
        label_text: String,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let items = self.current_items(prop, cx);
        let (rows, most, add, remove) = if prop == "buttons" {
            (&self.buttons, 5, "button-add", "button-remove")
        } else {
            (&self.menu, 25, "menu-option-add", "menu-option-remove")
        };
        v_flex()
            .gap(px(6.))
            .child(label(label_text))
            .children(
                rows.iter()
                    .zip(&items)
                    .enumerate()
                    .map(|(ix, (row, item))| {
                        let link =
                            item.get("style").and_then(serde_json::Value::as_str) == Some("link");
                        v_flex()
                            .gap(px(4.))
                            .p(px(6.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .child(
                                h_flex()
                                    .gap(px(4.))
                                    .child(div().flex_1().child(Input::new(&row.label).small()))
                                    .children(row.style.as_ref().map(|style| {
                                        div().w(px(104.)).child(Select::new(style).small())
                                    }))
                                    .child(
                                        Button::new((remove, ix))
                                            .ghost()
                                            .xsmall()
                                            .label(t("option-remove"))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.edit_items(prop, true, cx, |items| {
                                                    if ix < items.len() {
                                                        items.remove(ix);
                                                    }
                                                });
                                            })),
                                    ),
                            )
                            .children(
                                row.url
                                    .as_ref()
                                    .filter(|_| link)
                                    .map(|url| Input::new(url).small()),
                            )
                    }),
            )
            .child(
                Button::new(add)
                    .xsmall()
                    .label(t(add))
                    .disabled(items.len() >= most)
                    .on_click(cx.listener(move |this, _, _, cx| this.add_item(prop, cx))),
            )
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
        // A license typed into project.toml by hand stays on offer (D-136).
        let current = state.project.manifest.bot.license.clone();
        let mut licenses: Vec<&str> = crate::export::LICENSES.to_vec();
        if !licenses.contains(&current.as_str()) {
            licenses.push(&current);
        }
        let license_at = licenses
            .iter()
            .position(|l| *l == current)
            .map(IndexPath::new);
        let licenses: Vec<SharedString> = licenses.into_iter().map(license_name).collect();
        let license = cx.new(|cx| SelectState::new(licenses, license_at, window, cx));
        let license_changes = cx.subscribe(&license, |this, _, event, cx| {
            let SelectEvent::Confirm(Some(label)) = event else {
                return;
            };
            let license = if *label == license_name("") {
                ""
            } else {
                label.as_ref()
            };
            this.state
                .update(cx, |state, cx| state.set_license(license, cx));
        });
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
            license,
            _changes: [token_changes, server_changes, license_changes],
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

    fn render_bot_form(
        bot: &BotForm,
        hosted: &crate::host::Hosted,
        published: &crate::publish::Published,
        cx: &Context<'_, Self>,
    ) -> impl IntoElement + use<> {
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
            .child(
                Button::new("connect-discord")
                    .label(t("guide-open"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state
                            .update(cx, |_, cx| cx.emit(crate::state::OpenGuide));
                    })),
            )
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
            .child(label(t("bot-license")))
            .child(Select::new(&bot.license).small())
            .children(crate::host::AVAILABLE.then(|| hosting_section(hosted, cx)))
            .children(
                published
                    .address
                    .is_some()
                    .then(|| cloud_section(published, cx)),
            )
    }
}

/// "Hosting on this PC" in the bot settings (D-142).
fn hosting_section(hosted: &crate::host::Hosted, cx: &Context<'_, Properties>) -> gpui_kit::Div {
    use rustbot_runner::service::State;
    let theme = cx.theme();
    let section = v_flex()
        .gap(px(6.))
        .pt(px(8.))
        .child(div().text_sm().child(t("host-section")));
    let Some(text) = hosted.label() else {
        return section
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(t("host-none")),
            )
            .child(
                Button::new("host-here")
                    .label(t("host"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        crate::host::host(this.state.clone(), window, cx);
                    })),
            );
    };
    // While Run tests the bot, its hosted copy must stay stopped (D-141).
    let switch = match hosted.state {
        _ if hosted.paused => None,
        Some(State::Running | State::Starting) => Some((t("stop"), false)),
        _ => Some((t("host-start"), true)),
    };
    section.child(text).child(
        h_flex()
            .gap(px(8.))
            .child(
                Button::new("host-update")
                    .primary()
                    .label(t("host-update"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        crate::host::host(this.state.clone(), window, cx);
                    })),
            )
            .children(switch.map(|(label, on)| {
                Button::new("host-switch")
                    .label(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        crate::host::switch(&this.state, on, cx);
                    }))
            }))
            .child(
                Button::new("host-remove")
                    .label(t("host-remove-button"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        crate::host::remove(this.state.clone(), window, cx);
                    })),
            ),
    )
}

/// "RustBot Cloud" in the bot settings (D-171, D-174): signing in, publishing, the
/// published bot's state and its memory backups.
fn cloud_section(
    published: &crate::publish::Published,
    cx: &Context<'_, Properties>,
) -> gpui_kit::Div {
    let theme = cx.theme();
    let hint = |text: String| {
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(text)
    };
    let section = v_flex()
        .gap(px(6.))
        .pt(px(8.))
        .child(div().text_sm().child(t("cloud-section")));
    if !published.signed_in() {
        return cloud_sign_in(section, published, cx);
    }
    let who = published.user.as_ref().map_or_else(
        || t("cloud-signed-in-unknown"),
        |name| t_with("cloud-signed-in-as", &[("name", name)]),
    );
    let section = section.child(
        h_flex().gap(px(8.)).child(hint(who)).child(
            Button::new("cloud-sign-out")
                .ghost()
                .xsmall()
                .label(t("cloud-sign-out"))
                .on_click(cx.listener(|this, _, _, cx| {
                    crate::publish::sign_out(&this.state, cx);
                })),
        ),
    );
    let Some(label) = published.label() else {
        return section.child(hint(t("cloud-none"))).child(
            Button::new("cloud-publish")
                .primary()
                .label(t("cloud-publish"))
                .on_click(cx.listener(|this, _, window, cx| {
                    crate::publish::publish(this.state.clone(), window, cx);
                })),
        );
    };
    let memory = published
        .bot
        .as_ref()
        .and_then(|bot| bot.memory_kb)
        .map(|kb| {
            t_with(
                "cloud-memory",
                &[("mb", &format!("{}.{}", kb / 1024, kb % 1024 * 10 / 1024))],
            )
        });
    // While Run tests the bot, its Cloud copy must stay stopped (D-173).
    let switch = if published.paused {
        None
    } else if published.running() {
        Some((t("stop"), false))
    } else {
        Some((t("host-start"), true))
    };
    let backups = cloud_backups(published, cx);
    let no_backups = backups.is_empty().then(|| hint(t("cloud-no-backups")));
    section
        .child(label)
        .children(memory.map(hint))
        .child(
            h_flex()
                .gap(px(8.))
                .child(
                    Button::new("cloud-update")
                        .primary()
                        .label(t("host-update"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            crate::publish::publish(this.state.clone(), window, cx);
                        })),
                )
                .children(switch.map(|(label, on)| {
                    Button::new("cloud-switch")
                        .label(label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            crate::publish::switch(this.state.clone(), on, window, cx);
                        }))
                }))
                .child(
                    Button::new("cloud-delete")
                        .label(t("cloud-delete-button"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            crate::publish::delete(this.state.clone(), window, cx);
                        })),
                ),
        )
        .child(div().text_xs().pt(px(4.)).child(t("cloud-backups")))
        .children(no_backups)
        .children(backups)
}

/// Signed out: what Cloud is, and "Sign in with Discord" (D-160).
fn cloud_sign_in(
    section: gpui_kit::Div,
    published: &crate::publish::Published,
    cx: &Context<'_, Properties>,
) -> gpui_kit::Div {
    let label = if published.signing_in {
        t("cloud-signing-in")
    } else {
        t("cloud-sign-in")
    };
    section
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(t("cloud-about")),
        )
        .child(
            Button::new("cloud-sign-in")
                .label(label)
                .disabled(published.signing_in)
                .on_click(cx.listener(|this, _, window, cx| {
                    crate::publish::sign_in(this.state.clone(), window, cx);
                })),
        )
}

/// The Cloud bot's memory backups, each with Restore and Download (D-174).
fn cloud_backups(
    published: &crate::publish::Published,
    cx: &Context<'_, Properties>,
) -> Vec<gpui_kit::Div> {
    published
        .backups
        .iter()
        .enumerate()
        .map(|(ix, backup)| {
            let (restore, download) = (backup.id.clone(), backup.id.clone());
            h_flex()
                .gap(px(8.))
                .child(div().text_xs().flex_1().child(format!(
                    "{} · {} KB",
                    backup_time(&backup.time),
                    backup.size.div_ceil(1024)
                )))
                .child(
                    Button::new(("cloud-restore", ix))
                        .ghost()
                        .xsmall()
                        .label(t("cloud-restore-button"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            crate::publish::restore(
                                this.state.clone(),
                                restore.clone(),
                                window,
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new(("cloud-download", ix))
                        .ghost()
                        .xsmall()
                        .label(t("cloud-download"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            crate::publish::download(
                                this.state.clone(),
                                download.clone(),
                                window,
                                cx,
                            );
                        })),
                )
        })
        .collect()
}

/// `2026-11-24T15:19:49Z` as `2026-11-24 15:19 UTC`.
fn backup_time(time: &str) -> String {
    match (time.get(..10), time.get(11..16)) {
        (Some(day), Some(clock)) => format!("{day} {clock} UTC"),
        _ => time.to_owned(),
    }
}

/// A license as the bot settings show it: its SPDX expression, or "None".
fn license_name(license: &str) -> SharedString {
    if license.is_empty() {
        t("license-none").into()
    } else {
        SharedString::from(license.to_owned())
    }
}

/// A header that opens and closes a section of the form, titled by the Fluent message
/// `id` (D-087, D-098).
fn section(
    id: &'static str,
    open: bool,
    cx: &Context<'_, Properties>,
    toggle: fn(&mut Properties),
) -> gpui_kit::AnyElement {
    let arrow = if open { "▾" } else { "▸" };
    Button::new(id)
        .ghost()
        .small()
        .label(format!("{arrow} {}", t(id)))
        .on_click(cx.listener(move |this, _, _, cx| {
            toggle(this);
            cx.notify();
        }))
        .into_any_element()
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

/// The variables a template of `node` can use, e.g. `{user.name}` or `{option.reason}`, and
/// the memory (D-108).
fn variables(file: &GraphFile, registry: &Registry, node: &NodeId) -> Vec<String> {
    let mut found = Vec::new();
    found.extend(
        registry
            .memory()
            .iter()
            .map(|var| format!("{{memory.{}}}", var.name)),
    );
    for variable in registry.scope_at(&file.graph, node) {
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

fn memory_type_name(ty: MemoryType) -> String {
    t(match ty {
        MemoryType::Number => "type-number",
        MemoryType::Text => "type-text",
    })
}

fn scope_name(scope: MemoryScope) -> String {
    t(match scope {
        MemoryScope::Global => "memory-scope-global",
        MemoryScope::Server => "memory-scope-server",
        MemoryScope::User => "memory-scope-user",
        MemoryScope::Member => "memory-scope-member",
    })
}

fn is_number_or_empty(text: &str) -> bool {
    text.trim().is_empty() || text.trim().parse::<f64>().is_ok_and(f64::is_finite)
}

/// A variable as a picker lists it: "Command author", "Option: target" (D-078).
fn variable_label(name: &str) -> String {
    match name {
        "user" => t("var-user"),
        "channel" => t("var-channel"),
        "server" => t("var-server"),
        "created_channel" => t("var-created-channel"),
        "created_thread" => t("var-created-thread"),
        "created_role" => t("var-created-role"),
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
        rustbot_core::registry::Choices::MessageTarget => "message_target",
        rustbot_core::registry::Choices::ChannelVisibility => "channel_visibility",
        rustbot_core::registry::Choices::ThreadStart => "thread_start",
        rustbot_core::registry::Choices::CodeResult => "code_result",
    };
    t_or(&format!("choice-{list}-{value}"), value)
}

/// A type's name in the app's language.
fn type_name(ty: DataType) -> SharedString {
    t(&format!("type-{}", data_type_value(ty))).into()
}

/// A type as graph files have it, e.g. `user`.
fn data_type_value(ty: DataType) -> &'static str {
    match ty {
        DataType::Text => "text",
        DataType::Number => "number",
        DataType::Bool => "bool",
        DataType::User => "user",
        DataType::Channel => "channel",
        DataType::Role => "role",
        DataType::Server => "server",
    }
}

/// The "Custom code" input types' names, in the order of [`CodeInput::TYPES`].
fn code_type_names() -> Vec<SharedString> {
    CodeInput::TYPES.iter().map(|ty| type_name(*ty)).collect()
}

fn code_type_index(value: &str) -> Option<usize> {
    CodeInput::TYPES
        .iter()
        .position(|ty| data_type_value(*ty) == value)
}

/// The option types' names, in the order of [`CommandOption::TYPES`].
fn type_names() -> Vec<SharedString> {
    CommandOption::TYPES
        .iter()
        .map(|ty| type_name(*ty))
        .collect()
}

/// The subscription that writes `key` of item `ix` of `prop` as `input` is typed in.
fn item_typing(
    prop: &'static str,
    ix: usize,
    key: &'static str,
    input: &Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<'_, Properties>,
) -> Subscription {
    cx.subscribe_in(
        input,
        window,
        move |this, input, event, _, cx| match event {
            InputEvent::Focus => this.fresh = true,
            InputEvent::Change => {
                let value = input.read(cx).value().to_string();
                let undoable = std::mem::take(&mut this.fresh);
                this.edit_items(prop, undoable, cx, |items| set_key(items, ix, key, value));
            }
            _ => {}
        },
    )
}

/// Sets `key` of item `ix`; an item that is not an object, from a hand-edited file, is
/// left alone, and Problems reports it.
fn set_key(
    items: &mut [serde_json::Value],
    ix: usize,
    key: &str,
    value: impl Into<serde_json::Value>,
) {
    if let Some(item) = items.get_mut(ix).and_then(serde_json::Value::as_object_mut) {
        item.insert(key.to_owned(), value.into());
    }
}

/// Drops the wires from outputs `node` no longer has, e.g. a removed button's.
fn prune_wires(file: &mut GraphFile, registry: &Registry, node: &NodeId) {
    let Some(shown) = file.graph.nodes.get(node) else {
        return;
    };
    let pins: Vec<String> = registry
        .pins(shown)
        .into_iter()
        .map(|pin| pin.name)
        .collect();
    file.graph
        .edges
        .retain(|edge| &edge.from.node != node || pins.contains(&edge.from.pin));
}

/// A button style as the graph file has it, e.g. `success`.
fn style_value(style: ButtonStyle) -> &'static str {
    match style {
        ButtonStyle::Primary => "primary",
        ButtonStyle::Secondary => "secondary",
        ButtonStyle::Success => "success",
        ButtonStyle::Danger => "danger",
        ButtonStyle::Link => "link",
    }
}

fn style_index(value: &str) -> Option<usize> {
    ButtonStyle::ALL
        .iter()
        .position(|style| style_value(*style) == value)
}

/// The button styles' names, in the order of [`ButtonStyle::ALL`].
fn style_names() -> Vec<SharedString> {
    ButtonStyle::ALL
        .iter()
        .map(|style| t(&format!("button-style-{}", style_value(*style))).into())
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
                let variables = variables(file, &state.registry, node);
                let this = cx.entity().downgrade();
                // Privileged intents are switched on by hand in the portal (D-095).
                let intents = state
                    .registry
                    .get(kind)
                    .map_or_else(Vec::new, |t| t.intents.clone());
                let notes: Vec<_> = [
                    ("GUILD_MEMBERS", "intent-note-members"),
                    ("MESSAGE_CONTENT", "intent-note-content"),
                ]
                .into_iter()
                .filter(|(intent, _)| intents.iter().any(|i| i == intent))
                .map(|(_, note)| {
                    v_flex()
                        .gap(px(2.))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(t(note))
                        .child(
                            Link::new(("intent-portal", note.len()))
                                .href("https://discord.com/developers/applications")
                                .child(t("intent-note-link")),
                        )
                })
                .collect();
                v_flex()
                    .gap(px(6.))
                    .child(div().text_sm().child(super::editor::node_name(kind)))
                    .children(self.rows(file, node, &node_id, &variables, &this, cx))
                    .children(notes)
                    .into_any_element()
            }
            _ => match &self.bot {
                Some(bot) => {
                    let app = self.state.read(cx);
                    Self::render_bot_form(bot, &app.hosted, &app.published, cx).into_any_element()
                }
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

    /// "Add input" adds a text input with a free name, and "Create function" writes the
    /// function with it into `src/lib.rs`, once (D-176).
    #[gpui_kit::test]
    fn custom_code_adds_inputs_and_creates_its_function(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let mut project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        let node = serde_json::json!({"type": "core.custom_code", "props": {
            "function": "give_points", "inputs": [], "result": "number"}});
        let n3 = NodeId("n3".to_owned());
        project.graphs[0]
            .graph
            .nodes
            .insert(n3.clone(), serde_json::from_value(node).unwrap());
        let lib = project.root.join("src/lib.rs");
        cx.update(gpui_kit::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, _| app.selected_node = Some(n3.clone()));
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.click("code-input-add", cx);
        });
        let inputs = cx.read(|cx| {
            let file = state.read(cx).open_file().unwrap();
            file.graph.nodes[&NodeId("n3".to_owned())].props["inputs"].clone()
        });
        assert_eq!(
            inputs,
            serde_json::json!([{"name": "input1", "type": "text", "value": ""}])
        );
        for _ in 0..2 {
            cx.update(|window, cx| {
                window.render_frame(cx);
                window.click("code-create", cx);
            });
        }
        let code = std::fs::read_to_string(&lib).unwrap();
        assert_eq!(
            code.matches("pub async fn give_points(").count(),
            1,
            "{code}"
        );
        assert!(code.contains("    input1: String,\n) -> Result<f64, djs_rs::HandlerError>"));
        let logs = cx.read(|cx| state.read(cx).logs.len());
        assert!(logs >= 2);
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
        // Typed key by key: a part of an ID keeps the field open (the owner's report).
        for (typed, value) in [
            ("1", "1"),
            ("2345", "12345"),
            ("6789012345678", "123456789012345678"),
        ] {
            cx.update(|window, cx| window.input(typed, cx));
            assert_eq!(prop(cx, "role"), value);
            assert!(
                cx.read(|cx| properties.read(cx).pickers[1].id_chosen),
                "{value}"
            );
        }

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
    /// The reply's embed fields stay folded until the header opens them, and the
    /// "only the author" switch writes `true`, as one undo step (D-086, D-087).
    #[gpui_kit::test]
    fn the_embed_section_folds_and_the_switch_writes(cx: &mut TestAppContext) {
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
        let shown = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| {
                window.render_frame(cx);
                properties.update(cx, |this, cx| {
                    let (file, node) = (
                        this.state.read(cx).open_file().unwrap().clone(),
                        NodeId("n2".to_owned()),
                    );
                    let this_entity = cx.entity().downgrade();
                    this.rows(&file, &node, "core-reply", &[], &this_entity, cx)
                        .len()
                })
            })
        };
        // Message, the switch and the folded "Embed" and "Buttons and menu" headers.
        assert_eq!(shown(cx), 4);
        cx.update(|window, cx| window.click("embed-section", cx));
        assert_eq!(shown(cx), 9);

        cx.update(|window, cx| {
            properties.update(cx, |this, cx| {
                this.fresh = true;
                this.write("private", true, cx);
            });
            window.render_frame(cx);
        });
        let private = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                file.graph.nodes[&NodeId("n2".to_owned())]
                    .props
                    .get("private")
                    .cloned()
            })
        };
        assert_eq!(private(cx), Some(serde_json::Value::Bool(true)));
        cx.update(|window, cx| {
            state.update(cx, |state, cx| state.undo(false, cx));
            window.render_frame(cx);
        });
        assert_eq!(private(cx), None);
    }

    /// "Buttons and menu" adds buttons, each an output of the node; removing one takes its
    /// wire along, and "only the author" fills in the answer for others (D-098, D-100).
    #[gpui_kit::test]
    fn buttons_add_outputs_and_take_their_wires_along(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (_properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, _| {
                app.selected_node = Some(NodeId("n2".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        let n2 = NodeId("n2".to_owned());
        cx.update(gpui_kit::test::TestWindowExt::render_frame);
        cx.update(|window, cx| window.click("clicks-section", cx));
        for _ in 0..2 {
            cx.update(|window, cx| window.click("button-add", cx));
        }
        cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                let node = &state.open_file().unwrap().graph.nodes[&n2];
                let pins: Vec<String> = state
                    .registry
                    .pins(node)
                    .into_iter()
                    .map(|pin| pin.name)
                    .collect();
                assert_eq!(pins, ["run", "then", "b1", "b2"]);
                state.edit(true, cx, |file, registry| {
                    let n3 = crate::canvas::add_node(file, registry, "core.reply", [0., 0.]);
                    file.graph.edges.push(rustbot_core::Edge {
                        from: rustbot_core::PinRef {
                            node: n2.clone(),
                            pin: "b2".to_owned(),
                        },
                        to: rustbot_core::PinRef {
                            node: n3,
                            pin: "run".to_owned(),
                        },
                    });
                });
            });
        });
        let graph = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| state.read(cx).open_file().unwrap().graph.clone())
        };
        assert_eq!(graph(cx).edges.len(), 2);
        cx.update(gpui_kit::test::TestWindowExt::render_frame);
        cx.update(|window, cx| window.click(("button-remove", 1_usize), cx));
        let after = graph(cx);
        assert_eq!(
            after.nodes[&n2].props["buttons"].as_array().unwrap().len(),
            1
        );
        assert_eq!(after.edges.len(), 1, "the wire from b2 went with it");

        // "private" is the first switch, "only_author" the second.
        cx.update(gpui_kit::test::TestWindowExt::render_frame);
        cx.update(|window, cx| window.click(("prop-check", 1_usize), cx));
        let props = graph(cx).nodes[&n2].props.clone();
        assert_eq!(props["only_author"], serde_json::Value::Bool(true));
        assert_eq!(props["not_yours"], crate::i18n::t("not-yours-default"));

        // One undo for the switch and its answer, one for the removal.
        cx.update(|window, cx| {
            state.update(cx, |state, cx| {
                state.undo(false, cx);
                state.undo(false, cx);
            });
            window.render_frame(cx);
        });
        let back = graph(cx);
        assert_eq!(back.nodes[&n2].props.get("only_author"), None);
        assert_eq!(
            back.nodes[&n2].props["buttons"].as_array().unwrap().len(),
            2
        );
        assert_eq!(back.edges.len(), 2);
    }

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

    /// "New variable…" makes a variable under a valid, free name and picks it; its
    /// settings show under the list, and a number refuses a starting text (D-109).
    #[gpui_kit::test]
    fn memory_variables_are_made_in_the_node(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, cx| {
                app.edit(false, cx, |file, registry| {
                    crate::canvas::add_node(file, registry, "core.memory_change", [0., 0.]);
                });
                app.selected_node = Some(NodeId("n3".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        let labels = cx.read(|cx| properties.read(cx).pickers[0].labels.clone());
        assert_eq!(labels, [crate::i18n::t("memory-new")]);
        for (name, error) in [("XP", true), ("xp", false)] {
            cx.update(|window, cx| {
                properties.update(cx, |this, cx| this.pick(0, 0, window, cx));
                window.render_frame(cx);
            });
            cx.update(|window, cx| {
                window.input(name, cx);
                window.press("enter", cx);
            });
            let failed = cx.read(|cx| {
                let new = properties.read(cx).new_memory.as_ref();
                new.is_some_and(|new| new.error.is_some())
            });
            assert_eq!(failed, error, "{name}");
        }
        cx.read(|cx| {
            let state = state.read(cx);
            let var = &state.project.manifest.memory[0];
            assert_eq!(
                (var.name.as_str(), var.ty),
                ("xp", rustbot_core::MemoryType::Number)
            );
            let node = &state.open_file().unwrap().graph.nodes[&NodeId("n3".to_owned())];
            assert_eq!(node.props["variable"], "xp");
        });
        assert!(cx.read(|cx| properties.read(cx).memory.is_some()));

        let initial = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| state.read(cx).project.manifest.memory[0].initial.clone())
        };
        cx.update(|window, cx| {
            let field = properties.read(cx).memory.as_ref().unwrap().initial.clone();
            field.update(cx, |input, cx| input.focus(window, cx));
            window.render_frame(cx);
        });
        cx.update(|window, cx| window.input("5", cx));
        assert_eq!(initial(cx), "5");
        cx.update(|window, cx| window.input("x", cx));
        assert_eq!(initial(cx), "5");
        assert!(cx.read(|cx| properties.read(cx).memory.as_ref().unwrap().bad_initial));

        // A node without a variable has no settings to show.
        cx.update(|_, cx| {
            state.update(cx, |state, cx| {
                state.selected_node = Some(NodeId("n1".to_owned()));
                cx.notify();
            });
        });
        assert!(cx.read(|cx| properties.read(cx).memory.is_none()));
    }

    /// "+ Add field" adds text fields with free names, up to 5; typing and the switches
    /// write them, and "Remove" takes one out (D-112).
    #[gpui_kit::test]
    fn form_fields_are_edited_in_the_form(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let project = crate::workspace::new_project(&dir.path().join("bot")).unwrap();
        cx.update(gpui_kit::init);
        let mut state = None;
        let (properties, cx) = cx.add_window_view(|window, cx| {
            let app = cx.new(|cx| AppState::new(project, cx));
            app.update(cx, |app, cx| {
                app.edit(false, cx, |file, registry| {
                    crate::canvas::add_node(file, registry, "core.show_form", [0., 0.]);
                });
                app.selected_node = Some(NodeId("n3".to_owned()));
            });
            state = Some(app.clone());
            Properties::new(app, window, cx)
        });
        let state = state.unwrap();
        let fields = |cx: &mut gpui_kit::VisualTestContext| {
            cx.read(|cx| {
                let file = state.read(cx).open_file().unwrap();
                rustbot_core::registry::FormField::list(
                    file.graph.nodes[&NodeId("n3".to_owned())]
                        .props
                        .get("fields"),
                )
            })
        };
        cx.update(gpui_kit::test::TestWindowExt::render_frame);
        for _ in 0..6 {
            cx.update(|window, cx| {
                window.click("field-add", cx);
                window.render_frame(cx);
            });
        }
        let names: Vec<String> = fields(cx).into_iter().map(|field| field.name).collect();
        assert_eq!(names, ["field1", "field2", "field3", "field4", "field5"]);
        assert!(fields(cx)[0].required);

        cx.update(|window, cx| {
            let input = properties.read(cx).form_fields[0].name.clone();
            input.update(cx, |input, cx| {
                input.set_value("", window, cx);
                input.focus(window, cx);
            });
            window.render_frame(cx);
        });
        cx.update(|window, cx| window.input("reason", cx));
        cx.update(|window, cx| {
            window.click(("field-long", 0_usize), cx);
            window.click(("field-remove", 4_usize), cx);
            window.render_frame(cx);
        });
        let first = &fields(cx)[0];
        assert_eq!((first.name.as_str(), first.long), ("reason", true));
        assert_eq!(fields(cx).len(), 4);
        assert_eq!(cx.read(|cx| properties.read(cx).form_fields.len()), 4);
    }
}
