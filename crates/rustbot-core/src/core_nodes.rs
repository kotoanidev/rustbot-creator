//! The built-in `core.` node types. The vertical slice needs two (D-031).

use crate::registry::{
    Category, DataType, Direction, NodeType, Pin, PinKind, Prop, PropKind, Variable,
};

pub(crate) fn all() -> Vec<NodeType> {
    vec![slash_command(), reply()]
}

fn slash_command() -> NodeType {
    NodeType {
        id: "core.slash_command".to_owned(),
        category: Category::Trigger,
        pins: vec![flow("then", Direction::Out)],
        props: vec![text("name", false), text("description", false)],
        provides: vec![
            var("user", DataType::User),
            var("channel", DataType::Channel),
            var("server", DataType::Server),
        ],
        // Interactions arrive whatever the intents are.
        intents: Vec::new(),
    }
}

fn reply() -> NodeType {
    NodeType {
        id: "core.reply".to_owned(),
        category: Category::Action,
        pins: vec![flow("run", Direction::In), flow("then", Direction::Out)],
        props: vec![text("content", true)],
        provides: Vec::new(),
        intents: Vec::new(),
    }
}

fn flow(name: &str, direction: Direction) -> Pin {
    Pin {
        name: name.to_owned(),
        direction,
        kind: PinKind::Flow,
    }
}

/// A required text field.
fn text(name: &str, templates: bool) -> Prop {
    Prop {
        name: name.to_owned(),
        kind: PropKind::Text { templates },
        required: true,
    }
}

fn var(name: &str, ty: DataType) -> Variable {
    Variable {
        name: name.to_owned(),
        ty,
    }
}
