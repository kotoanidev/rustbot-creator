//! The node registry (D-052): what each node type has, and graph validation against it.
//!
//! Node types are plain data. Built-ins are registered through [`Registry::register`], the
//! same call WASM plugins will use (D-034). The Rust code for each type comes from
//! `rustbot-codegen`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::template::{self, Segment, TemplateError};
use crate::{Graph, MemoryType, MemoryVar, Node, NodeId, PinRef};

/// What a node type is: its pins, its form fields, and what it provides and needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeType {
    /// The namespaced id: `core.reply` for built-ins, `<plugin id>.<name>` for plugins. Its
    /// labels are the Fluent keys `node-<id with '.' as '-'>` and `…-<prop>` (D-037).
    pub id: String,
    /// Where it is shown in the node palette.
    pub category: Category,
    /// Its pins.
    pub pins: Vec<Pin>,
    /// Its form fields.
    pub props: Vec<Prop>,
    /// For triggers: the variables templates can use in the graph (D-053).
    pub provides: Vec<Variable>,
    /// The gateway intents it needs, as djs-rs `Intents` names, e.g. `GUILD_MESSAGES`
    /// (D-049).
    pub intents: Vec<String>,
    /// For moderation actions: the Discord permission a member needs to have the bot do it
    /// to someone else, as a [`Choices::Permission`] value, e.g. `ban_members` (D-085).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,
    /// The triggers whose graphs it works in, e.g. only `core.slash_command`; empty for all
    /// (D-094).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<String>,
}

/// Where a node type is shown in the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    /// Starts a graph: a command, an event. A graph has exactly one.
    Trigger,
    /// Does something on Discord.
    Action,
    /// Branches, loops, conditions.
    Logic,
    /// Computes values for data pins.
    Data,
}

/// A pin of a node type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    /// Its name, as in `n1.then`.
    pub name: String,
    /// Whether wires come in or go out.
    pub direction: Direction,
    /// Execution order, or a value of some type.
    pub kind: PinKind,
}

/// Whether a pin is an input or an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    /// Wires come in.
    In,
    /// Wires go out.
    Out,
}

/// What travels along a wire (D-047).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinKind {
    /// Execution order.
    Flow,
    /// A value.
    Data(DataType),
}

/// The type of a value on a data pin or in a template variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataType {
    /// Text.
    Text,
    /// A number.
    Number,
    /// Yes or no.
    Bool,
    /// A Discord user.
    User,
    /// A channel.
    Channel,
    /// A server (guild).
    Server,
    /// A role on a server.
    Role,
}

impl DataType {
    /// The fields a template can read, e.g. `name` in `{user.name}`. Empty for plain values,
    /// which are used whole (`{option.text}`).
    #[must_use]
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Text | Self::Number | Self::Bool => &[],
            // `name` is what people see in the chat (D-054).
            Self::User => &["id", "name", "username", "display_name", "mention"],
            Self::Channel | Self::Role => &["id", "name", "mention"],
            Self::Server => &["id", "name"],
        }
    }
}

/// A form field of a node type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prop {
    /// Its key in the graph file's `props`.
    pub name: String,
    /// What it holds.
    pub kind: PropKind,
    /// Whether the graph is invalid without it.
    pub required: bool,
}

/// What a form field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropKind {
    /// Text, with `{variable}` templates if `templates` is true (D-047).
    Text {
        /// Whether `{…}` is a variable.
        templates: bool,
    },
    /// A slash command's options: a list of [`CommandOption`] (D-076).
    Options,
    /// A user, channel or role, as the name of a variable of this type in scope, e.g.
    /// `option.target` (D-078). A role may also be an ID (D-080).
    Variable(DataType),
    /// One value of a fixed list.
    Choice(Choices),
    /// A number above 0 and at most `max`, or a `{variable}` template that gives one when
    /// the bot runs (D-084).
    Number {
        /// The largest number allowed.
        max: u32,
    },
    /// A switch, `true` or `false`.
    Bool,
    /// A colour as `#RRGGBB`, or a `{variable}` template that gives one when the bot runs.
    Color,
    /// A message's buttons: a list of [`Button`] (D-098).
    Buttons,
    /// A message's select menu: a list of [`MenuOption`] (D-099).
    MenuOptions,
    /// A memory variable of the project, by its name (D-106).
    Memory,
    /// A form's text fields: a list of [`FormField`] (D-112).
    FormFields,
}

impl PropKind {
    /// What a new node holds in this field: nothing chosen or typed yet.
    #[must_use]
    pub fn default_value(self) -> serde_json::Value {
        match self {
            Self::Options | Self::Buttons | Self::MenuOptions | Self::FormFields => {
                serde_json::Value::Array(Vec::new())
            }
            // Deleting nothing is the safe start (D-083).
            Self::Choice(Choices::DeleteMessages) => serde_json::Value::String("none".to_owned()),
            // The event's message exists only in some graphs.
            Self::Choice(Choices::MessageTarget | Choices::ThreadStart) => {
                serde_json::Value::String("last".to_owned())
            }
            Self::Choice(Choices::ChannelVisibility) => {
                serde_json::Value::String("category".to_owned())
            }
            Self::Bool => serde_json::Value::Bool(false),
            Self::Text { .. }
            | Self::Variable(_)
            | Self::Choice(_)
            | Self::Number { .. }
            | Self::Color
            | Self::Memory => serde_json::Value::String(String::new()),
        }
    }
}

/// The fixed lists a [`PropKind::Choice`] field picks from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Choices {
    /// The Discord permissions a bot most often checks, by their lower-case names.
    Permission,
    /// How "Compare" relates two values (D-081).
    Comparison,
    /// How far back "Ban" deletes the user's messages (D-083).
    DeleteMessages,
    /// Which message a message action works on (D-103).
    MessageTarget,
    /// Who sees a channel "Create channel" makes (D-121).
    ChannelVisibility,
    /// What "Create thread" starts from: a message, or none in a channel (D-119).
    ThreadStart,
}

impl Choices {
    /// The values, in the order a form lists them.
    #[must_use]
    pub fn values(self) -> &'static [&'static str] {
        match self {
            Self::Permission => &[
                "administrator",
                "manage_guild",
                "manage_roles",
                "manage_channels",
                "manage_messages",
                "manage_nicknames",
                "kick_members",
                "ban_members",
                "moderate_members",
                "mute_members",
                "move_members",
                "mention_everyone",
                "view_audit_log",
            ],
            Self::Comparison => &["equals", "not_equals", "contains", "greater", "less"],
            Self::DeleteMessages => &["none", "hour", "day", "week"],
            Self::MessageTarget => &["last", "event"],
            Self::ChannelVisibility => &["category", "private"],
            Self::ThreadStart => &["last", "event", "channel"],
        }
    }
}

/// Whether a message node has nothing to send: no text, embed title or embed description
/// (D-087). Message nodes are those with the `embed_…` fields.
fn is_empty_message(node: &Node) -> bool {
    ["content", "embed_title", "embed_description"]
        .iter()
        .all(|name| {
            node.props
                .get(*name)
                .and_then(serde_json::Value::as_str)
                .is_none_or(|text| text.trim().is_empty())
        })
}

/// Whether a role or channel field's value is an ID rather than a variable: 17-20 digits
/// that fit a Discord ID (D-080, D-088).
#[must_use]
pub fn is_id(value: &str) -> bool {
    (17..=20).contains(&value.len())
        && value.bytes().all(|b| b.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|id| id != 0)
}

/// An option of a slash command, which users fill in, as stored in the graph file (D-076).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandOption {
    /// Its name, which is also the variable `{option.<name>}`.
    pub name: String,
    /// What users enter: one of [`CommandOption::TYPES`].
    #[serde(rename = "type")]
    pub ty: DataType,
    /// The hint Discord shows.
    pub description: String,
    /// Whether users must fill it in.
    #[serde(default)]
    pub required: bool,
}

impl CommandOption {
    /// The types an option can have.
    pub const TYPES: [DataType; 6] = [
        DataType::Text,
        DataType::Number,
        DataType::Bool,
        DataType::User,
        DataType::Channel,
        DataType::Role,
    ];

    /// The options stored in a form field, or none if it holds something else.
    #[must_use]
    pub fn list(value: Option<&serde_json::Value>) -> Vec<Self> {
        value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }
}

/// What is wrong with a command option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionProblem {
    /// Not 1-32 lower-case letters, digits, `-` or `_`.
    Name,
    /// Another option has the same name.
    Duplicate,
    /// Not 1-100 characters.
    Description,
    /// A type an option cannot have.
    Type,
    /// A required option after an optional one, which Discord refuses.
    RequiredAfterOptional,
    /// More than 25 options; reported on the 26th.
    TooMany,
}

/// A text field of a form (D-112), as stored in the graph file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormField {
    /// Read as `{field.<name>}` (D-114), with the rules of a command option's name.
    pub name: String,
    /// What users see above it: 1-45 characters.
    pub label: String,
    /// A paragraph rather than one line.
    #[serde(default)]
    pub long: bool,
    /// Whether it must be filled in.
    #[serde(default)]
    pub required: bool,
    /// Grey text in the empty field: at most 100 characters.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub placeholder: String,
    /// The fewest characters it takes, 0-4000; empty for no limit.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub min: String,
    /// The most characters it takes, 1-4000; empty for Discord's 4000.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub max: String,
}

impl FormField {
    /// The fields stored in a form field, or none if it holds something else.
    #[must_use]
    pub fn list(value: Option<&serde_json::Value>) -> Vec<Self> {
        value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }

    /// `min` or `max` as a number; `None` when empty, `Some(None)` when it is no length.
    #[must_use]
    pub fn length(text: &str) -> Option<Option<u16>> {
        let text = text.trim();
        (!text.is_empty()).then(|| text.parse().ok().filter(|n| *n <= 4000))
    }
}

/// What breaks Discord's rules for a form (D-112).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormProblem {
    /// The title is over 45 characters.
    Title,
    /// No fields, or more than 5; reported on the sixth.
    Count,
    /// A name that is not a valid one, or used twice.
    Name,
    /// A label that is empty or over 45 characters.
    Label,
    /// A hint over 100 characters.
    Placeholder,
    /// A minimum or maximum that is not 0-4000 (1-4000 for the maximum), or a minimum
    /// above the maximum.
    Length,
}

/// A button under a message, as stored in the graph file (D-098).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    /// Its output pin: `b1`, `b2`, …, kept when other buttons go.
    pub id: String,
    /// The text on it.
    pub label: String,
    /// Its colour, or a link.
    pub style: ButtonStyle,
    /// For a link: the address it opens.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
}

impl Button {
    /// The buttons stored in a form field, or none if it holds something else.
    #[must_use]
    pub fn list(value: Option<&serde_json::Value>) -> Vec<Self> {
        value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }
}

/// A button's colour, as Discord names them, or a link, which has no output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ButtonStyle {
    /// Blurple.
    Primary,
    /// Grey.
    Secondary,
    /// Green.
    Success,
    /// Red.
    Danger,
    /// Opens an address.
    Link,
}

impl ButtonStyle {
    /// Every style, in the order a form lists them.
    pub const ALL: [Self; 5] = [
        Self::Primary,
        Self::Secondary,
        Self::Success,
        Self::Danger,
        Self::Link,
    ];
}

/// An option of a message's select menu, as stored in the graph file (D-099).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuOption {
    /// Its output pin: `o1`, `o2`, …, kept when other options go.
    pub id: String,
    /// The text users pick.
    pub label: String,
}

impl MenuOption {
    /// The options stored in a form field, or none if it holds something else.
    #[must_use]
    pub fn list(value: Option<&serde_json::Value>) -> Vec<Self> {
        value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }
}

/// What is wrong with a button or a menu option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentProblem {
    /// A button's label is not 1-80 characters, an option's not 1-100.
    Label,
    /// A link's address does not start with `https://` or `http://`.
    Url,
    /// Its id is not `b<number>` (a button) or `o<number>` (an option), or is taken.
    Id,
    /// More than 5 buttons or 25 options; reported on the first one too many.
    TooMany,
}

/// The ids of a message node's buttons and menu options that have an output (D-098,
/// D-099); links have none.
#[must_use]
pub fn click_items(node: &Node) -> Vec<String> {
    let buttons = Button::list(node.props.get("buttons"))
        .into_iter()
        .filter(|button| button.style != ButtonStyle::Link)
        .map(|button| button.id);
    let options = MenuOption::list(node.props.get("menu"))
        .into_iter()
        .map(|option| option.id);
    buttons.chain(options).collect()
}

/// Whether a message node waits for the click in the same handler (D-097).
#[must_use]
pub fn waits_for_click(node: &Node) -> bool {
    node.props
        .get("wait")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|wait| !wait.trim().is_empty())
}

/// What a click provides when it "always works" (D-097): the clicker, the channel and the
/// server, like a command.
fn click_scope() -> Vec<Variable> {
    [
        ("user", DataType::User),
        ("channel", DataType::Channel),
        ("server", DataType::Server),
    ]
    .into_iter()
    .map(|(name, ty)| Variable {
        name: name.to_owned(),
        ty,
    })
    .collect()
}

/// Where a node runs (D-097). After a click that "always works", a new handler starts in
/// which only [`click_scope`] is known (`routed`); after a click waited for, every variable
/// stays, and replies answer the click (`clicked`).
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::struct_excessive_bools)] // independent facts about where a node runs
struct Context {
    routed: bool,
    clicked: bool,
    /// The nodes earlier in the run that went on at "then", whose variables are known: the
    /// fields of a form (D-111) and what a node created (D-120).
    ran: Vec<NodeId>,
    /// After a form nobody sent: its interaction was answered with it, so nothing can
    /// reply.
    closed: bool,
    /// Something other than a condition ran since the interaction began, or it was
    /// answered: too late for a form, which must be the first answer, within 3 seconds
    /// (D-113).
    late: bool,
}

/// A variable a trigger provides to templates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    /// Its name, as in `{user.name}`.
    pub name: String,
    /// Its type, which decides its fields.
    pub ty: DataType,
}

/// [`Registry::register`] refused a node type.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    /// Another node type already has this id.
    #[error("node type {0:?} is already registered")]
    Duplicate(String),
    /// The id is not `<namespace>.<name>`.
    #[error("node type id {0:?} is not of the form `namespace.name`")]
    InvalidId(String),
}

/// Something wrong in a graph, for the "Problems" panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The node it is about, if any.
    pub node: Option<NodeId>,
    /// What is wrong.
    pub kind: ProblemKind,
}

/// What is wrong in a graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    /// The graph has no trigger node.
    NoTrigger,
    /// The graph has more than one trigger node.
    SeveralTriggers,
    /// A node's type is not registered, e.g. a plugin that is not installed.
    UnknownNodeType(String),
    /// A wire names a node or pin that does not exist.
    UnknownPin(PinRef),
    /// A wire starts at an input or ends at an output.
    WrongDirection(PinRef),
    /// A wire joins flow to data, or two different data types.
    KindMismatch {
        /// The output pin.
        from: PinRef,
        /// The input pin.
        to: PinRef,
    },
    /// A data input has more than one wire.
    SeveralSources(PinRef),
    /// A required form field is empty or missing.
    MissingProp(String),
    /// A form field the node type does not have.
    UnknownProp(String),
    /// A form field holds the wrong kind of value.
    WrongPropType(String),
    /// A template does not parse.
    BadTemplate {
        /// The form field.
        prop: String,
        /// Why.
        error: TemplateError,
    },
    /// A template uses a variable or field that does not exist.
    UnknownVariable {
        /// The form field.
        prop: String,
        /// The path, e.g. `user.nmae`.
        path: String,
    },
    /// A field names a variable of another type, e.g. a text option where a user is needed.
    WrongVariableType {
        /// The form field.
        prop: String,
        /// The variable, e.g. `option.reason`.
        path: String,
    },
    /// A warning, which does not block Run: an action that needs `permission` can run
    /// without "Has permission?" checking the command author first (D-085).
    NoPermissionCheck {
        /// The permission, as a [`Choices::Permission`] value.
        permission: String,
    },
    /// A warning: after a click waited for that anyone may make, an action that needs
    /// `permission` runs for whoever clicked; a check of `user` checks who started it, not
    /// the clicker, and Discord's gate covers only the command (D-085, D-100).
    AnyoneCanClick {
        /// The permission, as a [`Choices::Permission`] value.
        permission: String,
    },
    /// A warning: a path through the command can end without "Reply", so Discord shows
    /// "The application did not respond" (D-089).
    NoReply,
    /// The node does not work under this graph's trigger; it works under `allowed` (D-094).
    WrongTrigger {
        /// The trigger node types it works under.
        allowed: Vec<String>,
    },
    /// "Only the command author sees it" outside a command (D-086).
    PrivateOutsideCommand,
    /// A message has no text and no embed title or description (D-087).
    EmptyMessage,
    /// A colour field holds something other than `#RRGGBB`.
    BadColor(String),
    /// A number field holds something other than a number from above 0 to `max` (D-084).
    BadNumber {
        /// The form field.
        prop: String,
        /// The largest number allowed.
        max: u32,
    },
    /// A button or menu option breaks Discord's rules (D-098, D-099).
    BadComponent {
        /// The form field: `buttons` or `menu`.
        prop: String,
        /// Its position in the list, from 0.
        item: usize,
        /// What is wrong.
        problem: ComponentProblem,
    },
    /// "Only the command author can click" where no user started the graph (D-100).
    OnlyAuthorWithoutUser,
    /// "The event's message" where no message came with the event: outside "New message"
    /// and reactions, or after a click that always works (D-103).
    NoEventMessage,
    /// A memory variable the project does not declare (D-106).
    UnknownMemory(String),
    /// A form breaks Discord's rules (D-112).
    BadForm {
        /// The field's position from 0, or `None` for the title or the number of fields.
        field: Option<usize>,
        /// What is wrong.
        problem: FormProblem,
    },
    /// "Show form" after the bot did something other than a condition, or answered: a form
    /// must be the first answer to a command or click (D-113).
    FormNotFirst,
    /// "Reply" after a form nobody sent ("timeout"), when there is nothing to reply to.
    ReplyAfterClosedForm,
    /// A memory variable kept per server where no server is known, e.g. "Bot is ready".
    MemoryNeedsServer,
    /// A memory variable kept per user where no user is known and none is chosen.
    MemoryNeedsUser,
    /// A field that must hold a number, or a variable that is not a number for "Change by".
    NotANumber(String),
    /// "Ranking" of a variable that is not a number kept per user (D-122).
    RankingNotPerUser,
    /// A command option breaks Discord's rules.
    BadOption {
        /// Its position in the list, from 0.
        option: usize,
        /// What is wrong.
        problem: OptionProblem,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(node) = &self.node {
            write!(f, "{node}: ")?;
        }
        match &self.kind {
            ProblemKind::NoTrigger => f.write_str("the graph has no trigger"),
            ProblemKind::SeveralTriggers => f.write_str("the graph has more than one trigger"),
            ProblemKind::UnknownNodeType(id) => write!(f, "unknown node type {id:?}"),
            ProblemKind::UnknownPin(pin) => write!(f, "no pin {pin}"),
            ProblemKind::WrongDirection(pin) => write!(f, "pin {pin} points the wrong way"),
            ProblemKind::KindMismatch { from, to } => {
                write!(f, "{from} and {to} carry different things")
            }
            ProblemKind::SeveralSources(pin) => write!(f, "{pin} has more than one wire"),
            ProblemKind::MissingProp(prop) => write!(f, "{prop:?} is required"),
            ProblemKind::UnknownProp(prop) => write!(f, "unknown field {prop:?}"),
            ProblemKind::WrongPropType(prop) => write!(f, "{prop:?} holds the wrong kind of value"),
            ProblemKind::BadTemplate { prop, error } => write!(f, "{prop:?}: {error}"),
            ProblemKind::UnknownVariable { prop, path } => {
                write!(f, "{prop:?}: no variable {{{path}}}")
            }
            ProblemKind::WrongVariableType { prop, path } => {
                write!(f, "{prop:?}: {{{path}}} has the wrong type")
            }
            ProblemKind::NoReply => write!(f, "the command may end without a reply"),
            ProblemKind::AnyoneCanClick { permission } => write!(
                f,
                "anyone who clicks can make an action needing {permission} run"
            ),
            ProblemKind::EmptyMessage => write!(f, "the message has no text and no embed"),
            ProblemKind::WrongTrigger { allowed } => {
                write!(f, "the node works only under {}", allowed.join(", "))
            }
            ProblemKind::PrivateOutsideCommand => {
                write!(f, "only a command's reply can be seen by its author alone")
            }
            ProblemKind::BadColor(prop) => write!(f, "{prop:?} is not a colour like #5865F2"),
            ProblemKind::NoPermissionCheck { permission } => {
                write!(
                    f,
                    "an action needing {permission} runs without a permission check"
                )
            }
            ProblemKind::BadNumber { prop, max } => {
                write!(f, "{prop:?} is not a number above 0 and at most {max}")
            }
            ProblemKind::BadComponent {
                prop,
                item,
                problem,
            } => {
                let why = match problem {
                    ComponentProblem::Label => "the label is empty or too long",
                    ComponentProblem::Url => "a link needs an address starting with https://",
                    ComponentProblem::Id => "its id is not unique or not like b1 or o1",
                    ComponentProblem::TooMany => "a message has at most 5 buttons and 25 options",
                };
                write!(f, "{prop:?} {}: {why}", item + 1)
            }
            ProblemKind::OnlyAuthorWithoutUser => {
                f.write_str("only the author can click, but no user started this")
            }
            ProblemKind::NoEventMessage => f.write_str("this event comes with no message"),
            ProblemKind::UnknownMemory(name) => write!(f, "no memory variable {name:?}"),
            ProblemKind::BadForm { field, problem } => {
                write!(f, "the form breaks Discord's rules ({problem:?}")?;
                match field {
                    Some(field) => write!(f, " in field {})", field + 1),
                    None => f.write_str(")"),
                }
            }
            ProblemKind::FormNotFirst => {
                f.write_str("a form must be the first answer, with only conditions before it")
            }
            ProblemKind::ReplyAfterClosedForm => {
                f.write_str("nothing can be replied to after a form that was not sent")
            }
            ProblemKind::MemoryNeedsServer => {
                f.write_str("the variable is kept per server, and no server is known here")
            }
            ProblemKind::MemoryNeedsUser => {
                f.write_str("the variable is kept per user, and no user is known here")
            }
            ProblemKind::NotANumber(prop) => write!(f, "{prop:?} is not a number"),
            ProblemKind::RankingNotPerUser => {
                f.write_str("a ranking needs a number variable kept per user")
            }
            ProblemKind::BadOption { option, problem } => {
                let why = match problem {
                    OptionProblem::Name => "a name needs 1-32 lower-case letters, digits, - or _",
                    OptionProblem::Duplicate => "another option has this name",
                    OptionProblem::Description => "a description needs 1-100 characters",
                    OptionProblem::Type => "an option cannot have this type",
                    OptionProblem::RequiredAfterOptional => {
                        "a required option must come before the optional ones"
                    }
                    OptionProblem::TooMany => "a command has at most 25 options",
                };
                write!(f, "option {}: {why}", option + 1)
            }
        }
    }
}

/// Every node type the app knows, built-in and from plugins, and the open project's memory
/// variables, which graphs check against (D-106).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    types: BTreeMap<String, NodeType>,
    memory: Vec<MemoryVar>,
}

impl Registry {
    /// A registry with the built-in `core.` node types.
    #[must_use]
    pub fn with_core() -> Self {
        // The test `core_nodes_register` checks these the way `register` would.
        let types = crate::core_nodes::all()
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();
        Self {
            types,
            memory: Vec::new(),
        }
    }

    /// Sets the project's memory variables, which graphs are checked against.
    pub fn set_memory(&mut self, memory: Vec<MemoryVar>) {
        self.memory = memory;
    }

    /// The project's memory variables.
    #[must_use]
    pub fn memory(&self) -> &[MemoryVar] {
        &self.memory
    }

    /// Adds a node type.
    ///
    /// # Errors
    /// The id is taken, or is not `namespace.name`.
    pub fn register(&mut self, node_type: NodeType) -> Result<(), RegisterError> {
        let valid = node_type
            .id
            .split_once('.')
            .is_some_and(|(ns, name)| !ns.is_empty() && !name.is_empty());
        if !valid {
            return Err(RegisterError::InvalidId(node_type.id));
        }
        if self.types.contains_key(&node_type.id) {
            return Err(RegisterError::Duplicate(node_type.id));
        }
        self.types.insert(node_type.id.clone(), node_type);
        Ok(())
    }

    /// The node type with this id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&NodeType> {
        self.types.get(id)
    }

    /// Every node type, sorted by id.
    pub fn iter(&self) -> impl Iterator<Item = &NodeType> {
        self.types.values()
    }

    /// The permission `node` needs, if it is a moderation action that acts on someone
    /// other than the command author (D-085). Acting on the author is allowed, so "/verify"
    /// can give its user a fixed role; a role from an option needs the permission even
    /// then, since the user could pick any role.
    #[must_use]
    pub fn needed_permission(&self, node: &Node) -> Option<&str> {
        let permission = self.get(&node.kind)?.permission.as_deref()?;
        let text = |name: &str| node.props.get(name).and_then(serde_json::Value::as_str);
        let on_others = text("user") != Some("user") || text("role").is_some_and(|r| !is_id(r));
        on_others.then_some(permission)
    }

    /// Warnings, which do not block Run: moderation actions that a path from the trigger
    /// reaches without passing the "yes" of "Has permission?" for the command author
    /// (D-085).
    #[must_use]
    pub fn warnings(&self, graph: &Graph) -> Vec<Problem> {
        let mut warnings = Vec::new();
        let Some(trigger) = self.trigger(graph) else {
            return warnings;
        };
        if graph.nodes[trigger].kind == "core.slash_command"
            && !self.always_replies(graph, trigger, &mut Vec::new(), &mut BTreeMap::new())
        {
            warnings.push(Problem {
                node: Some(trigger.clone()),
                kind: ProblemKind::NoReply,
            });
        }
        // (node, guarded, anyone) triples, so loops and joins end. `anyone`: after a click
        // waited for that anyone may make, so `user` is not necessarily who clicked.
        let mut seen = BTreeSet::new();
        let mut next = vec![(trigger.clone(), false, false)];
        while let Some((id, guarded, anyone)) = next.pop() {
            if !seen.insert((id.clone(), guarded, anyone)) {
                continue;
            }
            let Some(node) = graph.nodes.get(&id) else {
                continue;
            };
            if let Some(permission) = self.needed_permission(node) {
                let permission = permission.to_owned();
                let kind = if anyone {
                    Some(ProblemKind::AnyoneCanClick { permission })
                } else {
                    (!guarded).then_some(ProblemKind::NoPermissionCheck { permission })
                };
                warnings.extend(kind.map(|kind| Problem {
                    node: Some(id.clone()),
                    kind,
                }));
            }
            let checks_author = node.kind == "core.has_permission"
                && node.props.get("user").and_then(serde_json::Value::as_str) == Some("user");
            let clicks = click_items(node);
            let waits = waits_for_click(node);
            let only_author = node.props.get("only_author") == Some(&serde_json::Value::Bool(true));
            for edge in graph.edges.iter().filter(|edge| edge.from.node == id) {
                let (guarded, anyone) = if !clicks.contains(&edge.from.pin) {
                    (guarded || (checks_author && edge.from.pin == "yes"), anyone)
                } else if !waits {
                    // A new run by the clicker, who is `user`; nothing was checked yet
                    // (D-097).
                    (false, false)
                } else if only_author {
                    (guarded, anyone)
                } else {
                    (false, true)
                };
                next.push((edge.to.node.clone(), guarded, anyone));
            }
        }
        warnings
    }

    /// Whether every way through the command from `id` on replies (D-089). Siblings on one
    /// pin run one after another, so one that always replies is enough; each output pin of
    /// a node is a branch, and all of them must reply. An unwired "error" ends the command
    /// with a failure in Logs (D-082) and is left out.
    fn always_replies(
        &self,
        graph: &Graph,
        id: &NodeId,
        path: &mut Vec<NodeId>,
        known: &mut BTreeMap<NodeId, bool>,
    ) -> bool {
        // Joins reach a node many ways; its answer is the same each time.
        if let Some(&replies) = known.get(id) {
            return replies;
        }
        let Some(node) = graph.nodes.get(id) else {
            return false;
        };
        // A form answers the command too (D-111).
        if matches!(node.kind.as_str(), "core.reply" | "core.show_form") {
            return true;
        }
        let Some(node_type) = self.get(&node.kind) else {
            return false;
        };
        if path.contains(id) {
            // A loop, which validation reports.
            return true;
        }
        path.push(id.clone());
        let replies = node_type
            .pins
            .iter()
            .filter(|pin| pin.direction == Direction::Out && pin.kind == PinKind::Flow)
            .all(|pin| {
                let next: Vec<&NodeId> = graph
                    .edges
                    .iter()
                    .filter(|edge| &edge.from.node == id && edge.from.pin == pin.name)
                    .map(|edge| &edge.to.node)
                    .collect();
                (pin.name == "error" && next.is_empty())
                    || next
                        .iter()
                        .any(|next| self.always_replies(graph, next, path, known))
            });
        path.pop();
        known.insert(id.clone(), replies);
        replies
    }

    /// Everything wrong in `graph`, in node order. Empty means it can be generated.
    #[must_use]
    pub fn validate(&self, graph: &Graph) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut report = |node: Option<&NodeId>, kind| {
            problems.push(Problem {
                node: node.cloned(),
                kind,
            });
        };

        let triggers: Vec<&NodeType> = graph
            .nodes
            .values()
            .filter_map(|node| self.get(&node.kind))
            .filter(|t| t.category == Category::Trigger)
            .collect();
        match triggers.len() {
            0 => report(None, ProblemKind::NoTrigger),
            1 => {}
            _ => report(None, ProblemKind::SeveralTriggers),
        }
        let trigger_id = self.trigger(graph);
        let trigger = trigger_id.map(|id| graph.nodes[id].kind.as_str());
        let contexts = self.contexts(graph, trigger_id);
        // A node no wire reaches is checked as if the trigger led to it.
        let unreached = BTreeSet::from([Context::default()]);
        for (id, node) in &graph.nodes {
            let Some(node_type) = self.get(&node.kind) else {
                report(Some(id), ProblemKind::UnknownNodeType(node.kind.clone()));
                continue;
            };
            // A node that runs in several contexts is checked in each, and every problem
            // is reported once.
            let mut found: Vec<ProblemKind> = Vec::new();
            for context in contexts.get(id).unwrap_or(&unreached) {
                let base = if context.routed {
                    Some("core.click")
                } else {
                    trigger
                };
                let scope = self.scope_in(graph, context);
                for kind in check_node(node, node_type, base, context, &scope, &self.memory) {
                    if !found.contains(&kind) {
                        found.push(kind);
                    }
                }
            }
            for kind in found {
                report(Some(id), kind);
            }
            for name in node.props.keys() {
                if !node_type.props.iter().any(|p| &p.name == name) {
                    report(Some(id), ProblemKind::UnknownProp(name.clone()));
                }
            }
            if node_type.props.iter().any(|p| p.name == "embed_title") && is_empty_message(node) {
                report(Some(id), ProblemKind::EmptyMessage);
            }
        }

        // Nodes on one output run one after another, so a form must be alone on its wire to
        // be the first answer (D-113).
        for edge in &graph.edges {
            let form = graph
                .nodes
                .get(&edge.to.node)
                .is_some_and(|node| node.kind == "core.show_form");
            if form && graph.edges.iter().filter(|e| e.from == edge.from).count() > 1 {
                report(Some(&edge.to.node), ProblemKind::FormNotFirst);
            }
        }

        let mut data_inputs = BTreeSet::new();
        for edge in &graph.edges {
            let (Some(from), Some(to)) = (self.pin(graph, &edge.from), self.pin(graph, &edge.to))
            else {
                for pin in [&edge.from, &edge.to] {
                    if self.pin(graph, pin).is_none() && self.is_known(graph, pin) {
                        report(Some(&pin.node), ProblemKind::UnknownPin(pin.clone()));
                    }
                }
                continue;
            };
            if from.direction != Direction::Out {
                report(
                    Some(&edge.from.node),
                    ProblemKind::WrongDirection(edge.from.clone()),
                );
            } else if to.direction != Direction::In {
                report(
                    Some(&edge.to.node),
                    ProblemKind::WrongDirection(edge.to.clone()),
                );
            } else if from.kind != to.kind {
                report(
                    Some(&edge.to.node),
                    ProblemKind::KindMismatch {
                        from: edge.from.clone(),
                        to: edge.to.clone(),
                    },
                );
            } else if matches!(to.kind, PinKind::Data(_)) && !data_inputs.insert(&edge.to) {
                report(
                    Some(&edge.to.node),
                    ProblemKind::SeveralSources(edge.to.clone()),
                );
            }
        }
        problems
    }

    /// The variables templates and fields can use in `graph`: what its trigger provides,
    /// and `option.<name>` for each command option (D-076). With several triggers, the
    /// first one counts.
    #[must_use]
    pub fn scope(&self, graph: &Graph) -> Vec<Variable> {
        let Some((node, node_type)) = graph.nodes.values().find_map(|node| {
            self.get(&node.kind)
                .filter(|t| t.category == Category::Trigger)
                .map(|t| (node, t))
        }) else {
            return Vec::new();
        };
        let mut scope = node_type.provides.clone();
        for prop in &node_type.props {
            if prop.kind == PropKind::Options {
                let options = CommandOption::list(node.props.get(&prop.name));
                scope.extend(options.into_iter().map(|option| Variable {
                    name: format!("option.{}", option.name),
                    ty: option.ty,
                }));
            }
        }
        scope
    }

    /// The pin a reference points to, if its node and pin exist.
    fn pin(&self, graph: &Graph, pin: &PinRef) -> Option<Pin> {
        let node = graph.nodes.get(&pin.node)?;
        self.pins(node).into_iter().find(|p| p.name == pin.pin)
    }

    /// A node's pins: its type's, an output per button and menu option of a message, and
    /// "timeout" when it waits for the click (D-097–D-099). Empty for an unknown type.
    #[must_use]
    pub fn pins(&self, node: &Node) -> Vec<Pin> {
        let Some(node_type) = self.get(&node.kind) else {
            return Vec::new();
        };
        let mut pins = node_type.pins.clone();
        let has_clicks = node_type
            .props
            .iter()
            .any(|p| matches!(p.kind, PropKind::Buttons | PropKind::MenuOptions));
        let clicks = if has_clicks {
            click_items(node)
        } else {
            Vec::new()
        };
        let timeout = (!clicks.is_empty() && waits_for_click(node)).then(|| "timeout".to_owned());
        pins.extend(clicks.into_iter().chain(timeout).map(|name| Pin {
            name,
            direction: Direction::Out,
            kind: PinKind::Flow,
        }));
        pins
    }

    /// The graph's trigger node; with several, the first.
    fn trigger<'a>(&self, graph: &'a Graph) -> Option<&'a NodeId> {
        graph.nodes.iter().find_map(|(id, node)| {
            self.get(&node.kind)
                .is_some_and(|t| t.category == Category::Trigger)
                .then_some(id)
        })
    }

    /// The contexts each node the trigger leads to runs in (D-097, D-111).
    fn contexts(
        &self,
        graph: &Graph,
        trigger: Option<&NodeId>,
    ) -> BTreeMap<NodeId, BTreeSet<Context>> {
        let mut reached: BTreeMap<NodeId, BTreeSet<Context>> = BTreeMap::new();
        let mut next: Vec<(NodeId, Context)> = trigger
            .map(|id| (id.clone(), Context::default()))
            .into_iter()
            .collect();
        while let Some((id, context)) = next.pop() {
            if !reached
                .entry(id.clone())
                .or_default()
                .insert(context.clone())
            {
                continue;
            }
            let Some(node) = graph.nodes.get(&id) else {
                continue;
            };
            let clicks = click_items(node);
            let waits = waits_for_click(node);
            // A trigger or a condition leaves time for a form; anything else does not.
            let quick = self.get(&node.kind).is_some_and(|t| {
                t.category == Category::Trigger || t.pins.iter().any(|pin| pin.name == "yes")
            });
            for edge in graph.edges.iter().filter(|edge| edge.from.node == id) {
                let after = if node.kind == "core.show_form" {
                    let mut after = Context {
                        late: true,
                        ..context.clone()
                    };
                    if edge.from.pin == "then" {
                        after.ran.push(id.clone());
                    } else {
                        after.closed = true;
                    }
                    after
                } else if !clicks.contains(&edge.from.pin) {
                    let mut after = Context {
                        late: context.late || !quick,
                        ..context.clone()
                    };
                    let provides = self
                        .get(&node.kind)
                        .is_some_and(|t| t.category != Category::Trigger && !t.provides.is_empty());
                    if provides && edge.from.pin == "then" {
                        after.ran.push(id.clone());
                    }
                    after
                } else if waits {
                    // A click is a new interaction to answer.
                    Context {
                        clicked: true,
                        closed: false,
                        late: false,
                        ..context.clone()
                    }
                } else {
                    Context {
                        routed: true,
                        ..Context::default()
                    }
                };
                next.push((edge.to.node.clone(), after));
            }
        }
        reached
    }

    /// The variables known in `context`: the graph's, or after a click that "always works"
    /// only the clicker, the channel and the server (D-097); then the fields of the forms
    /// sent before (D-114) and what nodes before created (D-120).
    fn scope_in(&self, graph: &Graph, context: &Context) -> Vec<Variable> {
        let mut scope = if context.routed {
            click_scope()
        } else {
            self.scope(graph)
        };
        for id in &context.ran {
            let Some(node) = graph.nodes.get(id) else {
                continue;
            };
            if let Some(node_type) = self.get(&node.kind) {
                scope.extend(node_type.provides.iter().cloned());
            }
            let fields = (node.kind == "core.show_form").then(|| node.props.get("fields"));
            scope.extend(
                FormField::list(fields.flatten())
                    .into_iter()
                    .map(|field| Variable {
                        name: format!("field.{}", field.name),
                        ty: DataType::Text,
                    }),
            );
        }
        scope
    }

    /// The variables `node`'s fields can use: the graph's, or after a click that "always
    /// works", only the clicker, the channel and the server (D-097); and after a form, its
    /// fields (D-114).
    #[must_use]
    pub fn scope_at(&self, graph: &Graph, node: &NodeId) -> Vec<Variable> {
        let contexts = self.contexts(graph, self.trigger(graph));
        let context = contexts
            .get(node)
            .and_then(|contexts| {
                contexts
                    .iter()
                    .find(|context| context.routed)
                    .or_else(|| contexts.iter().next())
            })
            .cloned()
            .unwrap_or_default();
        self.scope_in(graph, &context)
    }

    /// Whether the reference's node exists with a known type. Unknown types are reported once,
    /// on the node, rather than again on every wire.
    fn is_known(&self, graph: &Graph, pin: &PinRef) -> bool {
        graph
            .nodes
            .get(&pin.node)
            .is_none_or(|node| self.get(&node.kind).is_some())
    }
}

/// What is wrong with a node where it runs: under `base`, a trigger or `core.click` (D-097).
fn check_node(
    node: &Node,
    node_type: &NodeType,
    base: Option<&str>,
    context: &Context,
    scope: &[Variable],
    memory: &[MemoryVar],
) -> Vec<ProblemKind> {
    let mut problems = Vec::new();
    // After any click, "Reply" and "Show form" answer it.
    let replies_to_click =
        context.clicked && matches!(node.kind.as_str(), "core.reply" | "core.show_form");
    if let Some(base) = base
        && !node_type.triggers.is_empty()
        && !node_type.triggers.iter().any(|t| t == base)
        && !replies_to_click
    {
        problems.push(ProblemKind::WrongTrigger {
            allowed: node_type.triggers.clone(),
        });
    }
    let interaction = context.clicked || matches!(base, Some("core.slash_command" | "core.click"));
    if node.props.get("private") == Some(&serde_json::Value::Bool(true))
        && base.is_some()
        && !interaction
    {
        problems.push(ProblemKind::PrivateOutsideCommand);
    }
    for prop in &node_type.props {
        problems.extend(check_prop(prop, node.props.get(&prop.name), scope, memory));
    }
    problems.extend(check_memory_node(node, scope, memory));
    if node.kind == "core.show_form" {
        if context.late {
            problems.push(ProblemKind::FormNotFirst);
        }
        let title = node.props.get("title").and_then(serde_json::Value::as_str);
        if title.is_some_and(|title| title.trim().chars().count() > 45) {
            problems.push(ProblemKind::BadForm {
                field: None,
                problem: FormProblem::Title,
            });
        }
    }
    let text = |name: &str| node.props.get(name).and_then(serde_json::Value::as_str);
    if node.kind == "core.create_thread"
        && text("message") == Some("channel")
        && text("channel").is_none_or(str::is_empty)
    {
        problems.push(ProblemKind::MissingProp("channel".to_owned()));
    }
    if context.closed && node.kind == "core.reply" {
        problems.push(ProblemKind::ReplyAfterClosedForm);
    }
    let event_message = matches!(
        base,
        Some("core.message" | "core.reaction_add" | "core.reaction_remove")
    );
    if node
        .props
        .get("message")
        .and_then(serde_json::Value::as_str)
        == Some("event")
        && base.is_some()
        && !event_message
    {
        problems.push(ProblemKind::NoEventMessage);
    }
    if node.props.get("only_author") == Some(&serde_json::Value::Bool(true))
        && !click_items(node).is_empty()
        && !scope.iter().any(|var| var.name == "user")
    {
        problems.push(ProblemKind::OnlyAuthorWithoutUser);
    }
    problems
}

/// What is wrong with a node changing memory (D-108): whose value it is, and its numbers.
fn check_memory_node(node: &Node, scope: &[Variable], memory: &[MemoryVar]) -> Vec<ProblemKind> {
    let text = |name: &str| node.props.get(name).and_then(serde_json::Value::as_str);
    let Some(var) = text("variable").and_then(|name| memory.iter().find(|m| m.name == name)) else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    let known = |name: &str| scope.iter().any(|v| v.name == name);
    if var.scope.has_server() && !known("server") {
        problems.push(ProblemKind::MemoryNeedsServer);
    }
    // A ranking ranks every user, so it needs no one user (D-122).
    if node.kind == "core.ranking" {
        if var.ty != MemoryType::Number || !var.scope.has_user() {
            problems.push(ProblemKind::RankingNotPerUser);
        }
        return problems;
    }
    if var.scope.has_user() && text("user").is_none_or(str::is_empty) && !known("user") {
        problems.push(ProblemKind::MemoryNeedsUser);
    }
    // A number typed in must be one; a template is checked when the bot runs.
    let not_number = |value: &str| !value.contains('{') && value.trim().parse::<f64>().is_err();
    match (node.kind.as_str(), var.ty) {
        ("core.memory_change", MemoryType::Text) => {
            problems.push(ProblemKind::NotANumber("variable".to_owned()));
        }
        ("core.memory_change", MemoryType::Number)
            if not_number(text("amount").unwrap_or_default()) =>
        {
            problems.push(ProblemKind::NotANumber("amount".to_owned()));
        }
        ("core.memory_set", MemoryType::Number)
            if not_number(text("value").unwrap_or_default()) =>
        {
            problems.push(ProblemKind::NotANumber("value".to_owned()));
        }
        _ => {}
    }
    problems
}

/// What is wrong with one form field's value.
fn check_prop(
    prop: &Prop,
    value: Option<&serde_json::Value>,
    scope: &[Variable],
    memory: &[MemoryVar],
) -> Vec<ProblemKind> {
    let wrong_type = || vec![ProblemKind::WrongPropType(prop.name.clone())];
    match (prop.kind, value) {
        (_, None) if prop.required => vec![ProblemKind::MissingProp(prop.name.clone())],
        (_, None) | (PropKind::Bool, Some(serde_json::Value::Bool(_))) => Vec::new(),
        (PropKind::Text { templates }, Some(serde_json::Value::String(text))) => {
            if prop.required && text.trim().is_empty() {
                vec![ProblemKind::MissingProp(prop.name.clone())]
            } else if templates {
                check_template(&prop.name, text, scope, memory)
            } else {
                Vec::new()
            }
        }
        (PropKind::FormFields, Some(value)) => {
            serde_json::from_value::<Vec<FormField>>(value.clone())
                .map_or_else(|_| wrong_type(), |fields| check_form_fields(&fields))
        }
        (PropKind::Buttons, Some(value)) => serde_json::from_value::<Vec<Button>>(value.clone())
            .map_or_else(|_| wrong_type(), |buttons| check_buttons(&buttons)),
        (PropKind::MenuOptions, Some(value)) => {
            serde_json::from_value::<Vec<MenuOption>>(value.clone())
                .map_or_else(|_| wrong_type(), |options| check_menu(&options))
        }
        (PropKind::Options, Some(value)) => {
            serde_json::from_value::<Vec<CommandOption>>(value.clone())
                .map_or_else(|_| wrong_type(), |options| check_options(&options))
        }
        (
            PropKind::Variable(_) | PropKind::Choice(_) | PropKind::Memory,
            Some(serde_json::Value::String(text)),
        ) if text.is_empty() => {
            if prop.required {
                vec![ProblemKind::MissingProp(prop.name.clone())]
            } else {
                Vec::new()
            }
        }
        (PropKind::Memory, Some(serde_json::Value::String(name))) => {
            if memory.iter().any(|var| &var.name == name) {
                Vec::new()
            } else {
                vec![ProblemKind::UnknownMemory(name.clone())]
            }
        }
        (PropKind::Variable(ty), Some(serde_json::Value::String(name))) => {
            if matches!(ty, DataType::Role | DataType::Channel) && is_id(name) {
                return Vec::new();
            }
            match scope.iter().find(|var| &var.name == name) {
                Some(var) if var.ty == ty => Vec::new(),
                Some(_) => vec![ProblemKind::WrongVariableType {
                    prop: prop.name.clone(),
                    path: name.clone(),
                }],
                None => vec![ProblemKind::UnknownVariable {
                    prop: prop.name.clone(),
                    path: name.clone(),
                }],
            }
        }
        (PropKind::Number { max }, Some(serde_json::Value::String(text))) => {
            if prop.required && text.trim().is_empty() {
                vec![ProblemKind::MissingProp(prop.name.clone())]
            } else if text.trim().is_empty() || text.contains('{') {
                // A template: its number is only known when the bot runs.
                check_template(&prop.name, text, scope, memory)
            } else if text
                .trim()
                .parse::<f64>()
                .is_ok_and(|n| n > 0.0 && n <= f64::from(max))
            {
                Vec::new()
            } else {
                vec![ProblemKind::BadNumber {
                    prop: prop.name.clone(),
                    max,
                }]
            }
        }
        (PropKind::Color, Some(serde_json::Value::String(text))) => {
            let hex = text.trim().strip_prefix('#').unwrap_or_default();
            if text.trim().is_empty() || text.contains('{') {
                check_template(&prop.name, text, scope, memory)
            } else if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                Vec::new()
            } else {
                vec![ProblemKind::BadColor(prop.name.clone())]
            }
        }
        (PropKind::Choice(choices), Some(serde_json::Value::String(value))) => {
            if choices.values().contains(&value.as_str()) {
                Vec::new()
            } else {
                wrong_type()
            }
        }
        (_, Some(_)) => wrong_type(),
    }
}

fn check_template(
    prop: &str,
    text: &str,
    scope: &[Variable],
    memory: &[MemoryVar],
) -> Vec<ProblemKind> {
    let segments = match template::parse(text) {
        Ok(segments) => segments,
        Err(error) => {
            return vec![ProblemKind::BadTemplate {
                prop: prop.to_owned(),
                error,
            }];
        }
    };
    segments
        .iter()
        .filter_map(|segment| match segment {
            Segment::Var(path) if !resolves(path, scope, memory) => {
                Some(ProblemKind::UnknownVariable {
                    prop: prop.to_owned(),
                    path: path.join("."),
                })
            }
            _ => None,
        })
        .collect()
}

/// Whether `user.name` names a variable in scope and one of its fields. Objects need a field;
/// plain values are used whole. A variable's name may have dots itself, as `option.target`.
/// `memory.<name>` reads a memory variable for who and where this runs, and
/// `memory.<name>.<user variable>` for another user (D-108, C-026).
fn resolves(path: &[String], scope: &[Variable], memory: &[MemoryVar]) -> bool {
    if path.first().is_some_and(|first| first == "memory") {
        let Some(var) = path
            .get(1)
            .and_then(|name| memory.iter().find(|m| &m.name == name))
        else {
            return false;
        };
        let known = |name: &str| scope.iter().any(|v| v.name == name);
        let user = match &path[2..] {
            [] => known("user"),
            rest => {
                let name = rest.join(".");
                var.scope.has_user()
                    && scope
                        .iter()
                        .any(|v| v.name == name && v.ty == DataType::User)
            }
        };
        return (!var.scope.has_user() || user)
            && (!var.scope.has_server() || known("server"))
            && (var.scope.has_user() || path.len() == 2);
    }
    scope.iter().any(|var| {
        let name: Vec<&str> = var.name.split('.').collect();
        if path.len() < name.len() || path.iter().zip(&name).any(|(a, b)| a != b) {
            return false;
        }
        let fields = var.ty.fields();
        match &path[name.len()..] {
            [] => fields.is_empty(),
            [field] => fields.contains(&field.as_str()),
            _ => false,
        }
    })
}

/// Discord's rules for a command's options.
fn check_options(options: &[CommandOption]) -> Vec<ProblemKind> {
    let mut problems = Vec::new();
    let mut bad = |option, problem| problems.push(ProblemKind::BadOption { option, problem });
    if options.len() > 25 {
        bad(25, OptionProblem::TooMany);
    }
    let mut names = BTreeSet::new();
    let mut optional_before = false;
    for (ix, option) in options.iter().enumerate() {
        if !valid_option_name(&option.name) {
            bad(ix, OptionProblem::Name);
        } else if !names.insert(option.name.as_str()) {
            bad(ix, OptionProblem::Duplicate);
        }
        let length = option.description.chars().count();
        if option.description.trim().is_empty() || length > 100 {
            bad(ix, OptionProblem::Description);
        }
        if !CommandOption::TYPES.contains(&option.ty) {
            bad(ix, OptionProblem::Type);
        }
        if option.required && optional_before {
            bad(ix, OptionProblem::RequiredAfterOptional);
        }
        optional_before |= !option.required;
    }
    problems
}

/// Discord's rules for a form's fields (D-112).
fn check_form_fields(fields: &[FormField]) -> Vec<ProblemKind> {
    let mut problems = Vec::new();
    let mut bad = |field, problem| problems.push(ProblemKind::BadForm { field, problem });
    if fields.is_empty() {
        bad(None, FormProblem::Count);
    }
    // `-` and `_` make the same Rust name in generated code.
    let mut names = BTreeSet::new();
    for (ix, field) in fields.iter().enumerate() {
        if ix == 5 {
            bad(Some(ix), FormProblem::Count);
        }
        if !valid_option_name(&field.name) || !names.insert(field.name.replace('-', "_")) {
            bad(Some(ix), FormProblem::Name);
        }
        if field.label.trim().is_empty() || field.label.trim().chars().count() > 45 {
            bad(Some(ix), FormProblem::Label);
        }
        if field.placeholder.trim().chars().count() > 100 {
            bad(Some(ix), FormProblem::Placeholder);
        }
        let (min, max) = (FormField::length(&field.min), FormField::length(&field.max));
        let wrong = matches!(min, Some(None))
            || matches!(max, Some(None | Some(0)))
            || matches!((min, max), (Some(Some(min)), Some(Some(max))) if min > max);
        if wrong {
            bad(Some(ix), FormProblem::Length);
        }
    }
    problems
}

/// Discord's rules for a message's buttons (D-098).
fn check_buttons(buttons: &[Button]) -> Vec<ProblemKind> {
    let labels = buttons.iter().map(|b| (b.id.as_str(), b.label.as_str()));
    let mut problems = check_items("buttons", 'b', labels, 5, 80);
    for (item, button) in buttons.iter().enumerate() {
        let url = button.url.trim();
        let is_url = (url.starts_with("https://") || url.starts_with("http://"))
            && url.len() <= 512
            && !url.contains(char::is_whitespace);
        if button.style == ButtonStyle::Link && !is_url {
            problems.push(ProblemKind::BadComponent {
                prop: "buttons".to_owned(),
                item,
                problem: ComponentProblem::Url,
            });
        }
    }
    problems
}

/// Discord's rules for a message's select menu (D-099).
fn check_menu(options: &[MenuOption]) -> Vec<ProblemKind> {
    let labels = options.iter().map(|o| (o.id.as_str(), o.label.as_str()));
    check_items("menu", 'o', labels, 25, 100)
}

/// The rules buttons and menu options share: how many, their ids and their labels.
fn check_items<'a>(
    prop: &str,
    prefix: char,
    items: impl Iterator<Item = (&'a str, &'a str)>,
    most: usize,
    longest: usize,
) -> Vec<ProblemKind> {
    let mut problems = Vec::new();
    let mut ids = BTreeSet::new();
    for (item, (id, label)) in items.enumerate() {
        let mut bad = |problem| {
            problems.push(ProblemKind::BadComponent {
                prop: prop.to_owned(),
                item,
                problem,
            });
        };
        if item == most {
            bad(ComponentProblem::TooMany);
        }
        let number = id.strip_prefix(prefix).unwrap_or_default();
        let id_ok = !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit());
        if !id_ok || !ids.insert(id) {
            bad(ComponentProblem::Id);
        }
        if label.trim().is_empty() || label.chars().count() > longest {
            bad(ComponentProblem::Label);
        }
    }
    problems
}

/// Whether `name` can be a slash command's name: the same rule as an option's (D-092).
#[must_use]
pub fn valid_command_name(name: &str) -> bool {
    valid_option_name(name)
}

/// Whether `name` may name a memory variable (D-106): as a command's name, so it reads
/// well in `{memory.<name>}` and never holds a `.`.
#[must_use]
pub fn valid_memory_name(name: &str) -> bool {
    valid_option_name(name)
}

/// 1-32 lower-case letters (or letters without case), digits, `-` or `_`, as Discord asks.
fn valid_option_name(name: &str) -> bool {
    (1..=32).contains(&name.chars().count())
        && name.chars().all(|c| {
            c == '-' || c == '_' || c.is_numeric() || (c.is_alphabetic() && !c.is_uppercase())
        })
}
