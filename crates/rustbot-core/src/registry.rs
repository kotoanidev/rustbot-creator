//! The node registry (D-052): what each node type has, and graph validation against it.
//!
//! Node types are plain data. Built-ins are registered through [`Registry::register`], the
//! same call WASM plugins will use (D-034). The Rust code for each type comes from
//! `rustbot-codegen`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::template::{self, Segment, TemplateError};
use crate::{Graph, Node, NodeId, PinRef};

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
}

impl PropKind {
    /// What a new node holds in this field: nothing chosen or typed yet.
    #[must_use]
    pub fn default_value(self) -> serde_json::Value {
        match self {
            Self::Options => serde_json::Value::Array(Vec::new()),
            // Deleting nothing is the safe start (D-083).
            Self::Choice(Choices::DeleteMessages) => serde_json::Value::String("none".to_owned()),
            Self::Bool => serde_json::Value::Bool(false),
            Self::Text { .. }
            | Self::Variable(_)
            | Self::Choice(_)
            | Self::Number { .. }
            | Self::Color => serde_json::Value::String(String::new()),
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
    /// A warning: a path through the command can end without "Reply", so Discord shows
    /// "The application did not respond" (D-089).
    NoReply,
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
            ProblemKind::EmptyMessage => write!(f, "the message has no text and no embed"),
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

/// Every node type the app knows, built-in and from plugins.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    types: BTreeMap<String, NodeType>,
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
        Self { types }
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
        let Some(trigger) = graph.nodes.iter().find_map(|(id, node)| {
            self.get(&node.kind)
                .is_some_and(|t| t.category == Category::Trigger)
                .then_some(id)
        }) else {
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
        // (node, guarded) pairs, so loops and joins end.
        let mut seen = BTreeSet::new();
        let mut next = vec![(trigger.clone(), false)];
        while let Some((id, guarded)) = next.pop() {
            if !seen.insert((id.clone(), guarded)) {
                continue;
            }
            let Some(node) = graph.nodes.get(&id) else {
                continue;
            };
            if !guarded && let Some(permission) = self.needed_permission(node) {
                warnings.push(Problem {
                    node: Some(id.clone()),
                    kind: ProblemKind::NoPermissionCheck {
                        permission: permission.to_owned(),
                    },
                });
            }
            let checks_author = node.kind == "core.has_permission"
                && node.props.get("user").and_then(serde_json::Value::as_str) == Some("user");
            for edge in graph.edges.iter().filter(|edge| edge.from.node == id) {
                let guarded = guarded || (checks_author && edge.from.pin == "yes");
                next.push((edge.to.node.clone(), guarded));
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
        if node.kind == "core.reply" {
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
        let scope = self.scope(graph);
        let scope = scope.as_slice();

        for (id, node) in &graph.nodes {
            let Some(node_type) = self.get(&node.kind) else {
                report(Some(id), ProblemKind::UnknownNodeType(node.kind.clone()));
                continue;
            };
            for prop in &node_type.props {
                for kind in check_prop(prop, node.props.get(&prop.name), scope) {
                    report(Some(id), kind);
                }
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
    fn pin<'a>(&'a self, graph: &Graph, pin: &PinRef) -> Option<&'a Pin> {
        let node = graph.nodes.get(&pin.node)?;
        self.get(&node.kind)?
            .pins
            .iter()
            .find(|p| p.name == pin.pin)
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

/// What is wrong with one form field's value.
fn check_prop(
    prop: &Prop,
    value: Option<&serde_json::Value>,
    scope: &[Variable],
) -> Vec<ProblemKind> {
    let wrong_type = || vec![ProblemKind::WrongPropType(prop.name.clone())];
    match (prop.kind, value) {
        (_, None) if prop.required => vec![ProblemKind::MissingProp(prop.name.clone())],
        (_, None) | (PropKind::Bool, Some(serde_json::Value::Bool(_))) => Vec::new(),
        (PropKind::Text { templates }, Some(serde_json::Value::String(text))) => {
            if prop.required && text.trim().is_empty() {
                vec![ProblemKind::MissingProp(prop.name.clone())]
            } else if templates {
                check_template(&prop.name, text, scope)
            } else {
                Vec::new()
            }
        }
        (PropKind::Options, Some(value)) => {
            serde_json::from_value::<Vec<CommandOption>>(value.clone())
                .map_or_else(|_| wrong_type(), |options| check_options(&options))
        }
        (PropKind::Variable(_) | PropKind::Choice(_), Some(serde_json::Value::String(text)))
            if text.is_empty() =>
        {
            if prop.required {
                vec![ProblemKind::MissingProp(prop.name.clone())]
            } else {
                Vec::new()
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
                check_template(&prop.name, text, scope)
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
                check_template(&prop.name, text, scope)
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

fn check_template(prop: &str, text: &str, scope: &[Variable]) -> Vec<ProblemKind> {
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
            Segment::Var(path) if !resolves(path, scope) => Some(ProblemKind::UnknownVariable {
                prop: prop.to_owned(),
                path: path.join("."),
            }),
            _ => None,
        })
        .collect()
}

/// Whether `user.name` names a variable in scope and one of its fields. Objects need a field;
/// plain values are used whole. A variable's name may have dots itself, as `option.target`.
fn resolves(path: &[String], scope: &[Variable]) -> bool {
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

/// 1-32 lower-case letters (or letters without case), digits, `-` or `_`, as Discord asks.
fn valid_option_name(name: &str) -> bool {
    (1..=32).contains(&name.chars().count())
        && name.chars().all(|c| {
            c == '-' || c == '_' || c.is_numeric() || (c.is_alphabetic() && !c.is_uppercase())
        })
}
