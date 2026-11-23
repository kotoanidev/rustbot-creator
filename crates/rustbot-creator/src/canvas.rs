//! The graph canvas's geometry and edits, without the UI: where nodes and pins are, what
//! is under the pointer, and adding, removing and wiring nodes. Positions are graph
//! coordinates in pixels, `[x, y]`, as in the layout file (D-048).

use rustbot_core::registry::{Direction, NodeType, Pin};
use rustbot_core::{Edge, GraphFile, Node, NodeId, PinRef, Registry};

/// A node's width.
pub(crate) const WIDTH: f32 = 190.;
/// The height of a node's title.
pub(crate) const HEADER: f32 = 30.;
/// The height of one row of pins.
pub(crate) const ROW: f32 = 24.;
/// How far from a pin's centre a click still hits it.
const PIN_REACH: f32 = 9.;

/// Where `id` is drawn: its layout position, or a column for nodes without one.
pub(crate) fn position(file: &GraphFile, id: &NodeId) -> [f32; 2] {
    file.layout.nodes.get(id).copied().unwrap_or_else(|| {
        let index = file
            .graph
            .nodes
            .keys()
            .position(|other| other == id)
            .unwrap_or(0);
        [
            40.,
            40. + 120. * f32::from(u16::try_from(index).unwrap_or(u16::MAX)),
        ]
    })
}

fn side(node: Option<&NodeType>, direction: Direction) -> Vec<&Pin> {
    node.map_or_else(Vec::new, |node| {
        node.pins
            .iter()
            .filter(|pin| pin.direction == direction)
            .collect()
    })
}

/// A node's size: a title over as many rows as its longer side of pins.
pub(crate) fn size(node: Option<&NodeType>) -> [f32; 2] {
    let rows = side(node, Direction::In)
        .len()
        .max(side(node, Direction::Out).len())
        .max(1);
    [
        WIDTH,
        HEADER + ROW * f32::from(u16::try_from(rows).unwrap_or(u16::MAX)) + 6.,
    ]
}

/// The pins on one side of a node, with their centres relative to the node.
pub(crate) fn pins(node: Option<&NodeType>, direction: Direction) -> Vec<(&Pin, [f32; 2])> {
    let x = if direction == Direction::In {
        0.
    } else {
        WIDTH
    };
    side(node, direction)
        .into_iter()
        .zip(0_u16..)
        .map(|(pin, row)| (pin, [x, HEADER + ROW * (f32::from(row) + 0.5)]))
        .collect()
}

/// The centre of a pin in graph coordinates.
pub(crate) fn pin_position(
    file: &GraphFile,
    registry: &Registry,
    pin: &PinRef,
) -> Option<[f32; 2]> {
    let node = file.graph.nodes.get(&pin.node)?;
    let at = position(file, &pin.node);
    let kind = registry.get(&node.kind);
    [Direction::In, Direction::Out]
        .into_iter()
        .flat_map(|direction| pins(kind, direction))
        .find(|(p, _)| p.name == pin.pin)
        .map(|(_, offset)| [at[0] + offset[0], at[1] + offset[1]])
}

/// What is under a point.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Hit {
    /// A pin, which starts a wire.
    Pin(PinRef, Pin),
    /// A node's body, which selects and moves it.
    Node(NodeId),
    /// Empty canvas.
    Empty,
}

/// What is under `point`. Later nodes are drawn on top, so they win.
pub(crate) fn hit(file: &GraphFile, registry: &Registry, point: [f32; 2]) -> Hit {
    for (id, node) in file.graph.nodes.iter().rev() {
        let at = position(file, id);
        let kind = registry.get(&node.kind);
        for direction in [Direction::In, Direction::Out] {
            for (pin, offset) in pins(kind, direction) {
                let dx = point[0] - (at[0] + offset[0]);
                let dy = point[1] - (at[1] + offset[1]);
                if dx * dx + dy * dy <= PIN_REACH * PIN_REACH {
                    let at = PinRef {
                        node: id.clone(),
                        pin: pin.name.clone(),
                    };
                    return Hit::Pin(at, pin.clone());
                }
            }
        }
        let [w, h] = size(kind);
        if (at[0]..=at[0] + w).contains(&point[0]) && (at[1]..=at[1] + h).contains(&point[1]) {
            return Hit::Node(id.clone());
        }
    }
    Hit::Empty
}

/// Whether the box from `min` to `max` touches `view`, `[left, top, right, bottom]`.
pub(crate) fn in_view(view: [f32; 4], min: [f32; 2], max: [f32; 2]) -> bool {
    let [left, top, right, bottom] = view;
    min[0] <= right && max[0] >= left && min[1] <= bottom && max[1] >= top
}

/// Whether a wire can join `a` and `b`: one output and one input of the same kind.
pub(crate) fn fits(a: &Pin, b: &Pin) -> bool {
    a.direction != b.direction && a.kind == b.kind
}

/// The first pin of `node` a wire from `pin` can reach.
pub(crate) fn fitting_pin<'a>(node: &'a NodeType, pin: &Pin) -> Option<&'a Pin> {
    node.pins.iter().find(|other| fits(pin, other))
}

/// A new node of type `kind` at `at`, with empty fields. Returns its id: the next free
/// `nN`.
pub(crate) fn add_node(
    file: &mut GraphFile,
    registry: &Registry,
    kind: &str,
    at: [f32; 2],
) -> NodeId {
    let next = file
        .graph
        .nodes
        .keys()
        .filter_map(|id| id.0.strip_prefix('n')?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let id = NodeId(format!("n{next}"));
    let props = registry.get(kind).map_or_else(Default::default, |node| {
        node.props
            .iter()
            .map(|prop| (prop.name.clone(), prop.kind.default_value()))
            .collect()
    });
    file.graph.nodes.insert(
        id.clone(),
        Node {
            kind: kind.to_owned(),
            props,
        },
    );
    file.layout.nodes.insert(id.clone(), at);
    id
}

/// Removes a node, its position and every wire to or from it.
pub(crate) fn remove_node(file: &mut GraphFile, id: &NodeId) {
    file.graph.nodes.remove(id);
    file.layout.nodes.remove(id);
    file.graph
        .edges
        .retain(|edge| &edge.from.node != id && &edge.to.node != id);
}

/// Wires two pins, whichever way round they were dragged. An input keeps one wire, so
/// the new one replaces the old.
pub(crate) fn connect(file: &mut GraphFile, (a, a_pin): (&PinRef, &Pin), b: &PinRef) {
    let (from, to) = if a_pin.direction == Direction::Out {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    };
    file.graph.edges.retain(|edge| edge.to != to);
    file.graph.edges.push(Edge { from, to });
}

#[cfg(test)]
mod tests {
    use rustbot_core::{Graph, GraphKind, Layout, Registry};

    use super::*;

    fn ping() -> GraphFile {
        GraphFile {
            kind: GraphKind::Command,
            name: "ping".to_owned(),
            graph: serde_json::from_str(include_str!("../templates/ping.json")).unwrap(),
            layout: serde_json::from_str(include_str!("../templates/ping.layout.json")).unwrap(),
        }
    }

    fn pin(node: &str, pin: &str) -> PinRef {
        PinRef {
            node: NodeId(node.to_owned()),
            pin: pin.to_owned(),
        }
    }

    #[test]
    fn pins_nodes_and_empty_space_are_hit() {
        let file = ping();
        let registry = Registry::with_core();
        // n1 (the slash command) is at [80, 120]; its `then` output is on its right edge.
        let then = pin_position(&file, &registry, &pin("n1", "then")).unwrap();
        assert_eq!(then, [80. + WIDTH, 120. + HEADER + ROW / 2.]);
        let Hit::Pin(at, found) = hit(&file, &registry, [then[0] - 3., then[1] + 2.]) else {
            panic!("the pin is missed");
        };
        assert_eq!((at, found.direction), (pin("n1", "then"), Direction::Out));
        assert_eq!(
            hit(&file, &registry, [120., 125.]),
            Hit::Node(NodeId("n1".to_owned()))
        );
        assert_eq!(hit(&file, &registry, [0., 0.]), Hit::Empty);
    }

    #[test]
    fn add_wire_and_remove() {
        let mut file = ping();
        let registry = Registry::with_core();
        let id = add_node(&mut file, &registry, "core.reply", [400., 300.]);
        assert_eq!(id, NodeId("n3".to_owned()));
        assert_eq!(file.layout.nodes[&id], [400., 300.]);
        assert_eq!(file.graph.nodes[&id].props["content"], "");

        // Dragged from the input end; an input keeps one wire.
        let reply = registry.get("core.reply").unwrap();
        let run = fitting_pin(reply, &registry.get("core.slash_command").unwrap().pins[0]).unwrap();
        assert_eq!(run.name, "run");
        connect(&mut file, (&pin("n3", "run"), run), &pin("n2", "then"));
        connect(&mut file, (&pin("n3", "run"), run), &pin("n1", "then"));
        let into_n3: Vec<_> = file
            .graph
            .edges
            .iter()
            .filter(|e| e.to.node == id)
            .collect();
        assert_eq!(into_n3.len(), 1);
        assert_eq!(into_n3[0].from, pin("n1", "then"));

        remove_node(&mut file, &NodeId("n1".to_owned()));
        assert_eq!(file.graph.edges, Vec::new());
        assert!(!file.layout.nodes.contains_key(&NodeId("n1".to_owned())));
    }

    #[test]
    fn only_what_touches_the_view_is_drawn() {
        let view = [0., 0., 100., 100.];
        assert!(in_view(view, [90., 90.], [200., 200.]));
        assert!(!in_view(view, [101., 0.], [200., 50.]));
        assert!(!in_view(view, [0., -50.], [50., -1.]));
    }

    #[test]
    fn nodes_without_a_position_get_one() {
        let mut file = ping();
        file.layout = Layout::default();
        assert_ne!(
            position(&file, &NodeId("n1".to_owned())),
            position(&file, &NodeId("n2".to_owned()))
        );
        let empty = GraphFile {
            graph: Graph::default(),
            ..ping()
        };
        assert_eq!(hit(&empty, &Registry::with_core(), [50., 50.]), Hit::Empty);
    }
}
