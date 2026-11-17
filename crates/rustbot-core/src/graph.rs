use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::SCHEMA;

// Maps are `BTreeMap`, never `serde_json::Map`: the app's dependency tree turns on
// serde_json's `preserve_order`, and sorted keys are what keep diffs deterministic.

/// Which folder of `graphs/` a graph lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphKind {
    /// `graphs/commands/`: one slash command per graph.
    Command,
    /// `graphs/events/`: one gateway event handler per graph.
    Event,
}

impl GraphKind {
    /// Every kind, in folder order.
    pub const ALL: [Self; 2] = [Self::Command, Self::Event];

    /// The folder name under `graphs/`.
    #[must_use]
    pub fn dir(self) -> &'static str {
        match self {
            Self::Command => "commands",
            Self::Event => "events",
        }
    }
}

/// A node's id within its graph: `n1`, `n2`, … It is assigned by the app and never changes
/// (D-051).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One graph file: the logic of a command or an event handler (D-047, D-051).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    /// The project format version.
    pub schema: u32,
    /// The nodes, by id.
    #[serde(default)]
    pub nodes: BTreeMap<NodeId, Node>,
    /// Wires between pins, flow and data alike. The node registry knows which is which.
    #[serde(default)]
    pub edges: Vec<Edge>,
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            nodes: BTreeMap::new(),
            edges: Vec::new(),
        }
    }
}

/// A node: its type and the values set in its form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// The namespaced node type: `core.reply` for built-ins, `<plugin id>.<name>` for
    /// plugins (D-034).
    #[serde(rename = "type")]
    pub kind: String,
    /// The form values. Text values may contain `{path}` templates (D-047).
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

/// A wire from an output pin to an input pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// The output pin.
    pub from: PinRef,
    /// The input pin.
    pub to: PinRef,
}

/// A pin of a node, written `node.pin`, e.g. `n1.then`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PinRef {
    /// The node.
    pub node: NodeId,
    /// The pin's name on that node.
    pub pin: String,
}

/// A pin reference that is not `node.pin`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a pin reference of the form `node.pin`")]
pub struct PinRefError(String);

impl FromStr for PinRef {
    type Err = PinRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once('.') {
            Some((node, pin)) if !node.is_empty() && !pin.is_empty() => Ok(Self {
                node: NodeId(node.to_owned()),
                pin: pin.to_owned(),
            }),
            _ => Err(PinRefError(s.to_owned())),
        }
    }
}

impl TryFrom<String> for PinRef {
    type Error = PinRefError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<PinRef> for String {
    fn from(pin: PinRef) -> Self {
        pin.to_string()
    }
}

impl fmt::Display for PinRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.node, self.pin)
    }
}

/// A graph's node positions on the canvas, kept apart from its logic (D-048).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    /// The project format version.
    pub schema: u32,
    /// Each node's `[x, y]` position.
    #[serde(default)]
    pub nodes: BTreeMap<NodeId, [f32; 2]>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            nodes: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PinRef;

    #[test]
    fn pin_refs() {
        let pin: PinRef = "n1.then".parse().unwrap();
        assert_eq!((pin.node.0.as_str(), pin.pin.as_str()), ("n1", "then"));
        assert_eq!(pin.to_string(), "n1.then");
        for bad in ["n1", ".then", "n1.", ""] {
            assert!(bad.parse::<PinRef>().is_err(), "{bad:?} should be rejected");
        }
    }
}
