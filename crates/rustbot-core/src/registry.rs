//! The node registry (D-052): what each node type has, and graph validation against it.
//!
//! Node types are plain data. Built-ins are registered through [`Registry::register`], the
//! same call WASM plugins will use (D-034). The Rust code for each type comes from
//! `rustbot-codegen`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::template::{self, Segment, TemplateError};
use crate::{Graph, NodeId, PinRef};

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
            Self::Channel => &["id", "name", "mention"],
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
        let scope: &[Variable] = triggers.first().map_or(&[], |t| &t.provides);

        for (id, node) in &graph.nodes {
            let Some(node_type) = self.get(&node.kind) else {
                report(Some(id), ProblemKind::UnknownNodeType(node.kind.clone()));
                continue;
            };
            for prop in &node_type.props {
                let value = node.props.get(&prop.name);
                let PropKind::Text { templates } = prop.kind;
                match value {
                    None if prop.required => {
                        report(Some(id), ProblemKind::MissingProp(prop.name.clone()));
                    }
                    None => {}
                    Some(serde_json::Value::String(text)) => {
                        if prop.required && text.trim().is_empty() {
                            report(Some(id), ProblemKind::MissingProp(prop.name.clone()));
                        } else if templates {
                            for kind in check_template(&prop.name, text, scope) {
                                report(Some(id), kind);
                            }
                        }
                    }
                    Some(_) => report(Some(id), ProblemKind::WrongPropType(prop.name.clone())),
                }
            }
            for name in node.props.keys() {
                if !node_type.props.iter().any(|p| &p.name == name) {
                    report(Some(id), ProblemKind::UnknownProp(name.clone()));
                }
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
/// plain values are used whole.
fn resolves(path: &[String], scope: &[Variable]) -> bool {
    let Some(var) = scope.iter().find(|v| v.name == path[0]) else {
        return false;
    };
    let fields = var.ty.fields();
    match &path[1..] {
        [] => fields.is_empty(),
        [field] => fields.contains(&field.as_str()),
        _ => false,
    }
}
