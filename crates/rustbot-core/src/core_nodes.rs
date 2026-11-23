//! The built-in `core.` node types.

use crate::registry::{
    Category, Choices, DataType, Direction, NodeType, Pin, PinKind, Prop, PropKind, Variable,
};

pub(crate) fn all() -> Vec<NodeType> {
    vec![
        slash_command(),
        reply(),
        has_permission(),
        has_role(),
        compare(),
        ban(),
        kick(),
        timeout(),
        role_action("core.add_role"),
        role_action("core.remove_role"),
    ]
}

fn slash_command() -> NodeType {
    NodeType {
        id: "core.slash_command".to_owned(),
        category: Category::Trigger,
        pins: vec![flow("then", Direction::Out)],
        props: vec![
            text("name", false),
            text("description", false),
            prop("options", PropKind::Options, false),
        ],
        provides: vec![
            var("user", DataType::User),
            var("channel", DataType::Channel),
            var("server", DataType::Server),
        ],
        // Interactions arrive whatever the intents are.
        intents: Vec::new(),
        permission: None,
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
        permission: None,
    }
}

/// A condition (D-077): it runs, and continues at "yes" or "no".
fn condition(id: &str, props: Vec<Prop>) -> NodeType {
    NodeType {
        id: id.to_owned(),
        category: Category::Logic,
        pins: vec![
            flow("run", Direction::In),
            flow("yes", Direction::Out),
            flow("no", Direction::Out),
        ],
        props,
        provides: Vec::new(),
        intents: Vec::new(),
        permission: None,
    }
}

fn has_permission() -> NodeType {
    condition(
        "core.has_permission",
        vec![
            prop("user", PropKind::Variable(DataType::User), true),
            prop("permission", PropKind::Choice(Choices::Permission), true),
        ],
    )
}

fn has_role() -> NodeType {
    condition(
        "core.has_role",
        vec![
            prop("user", PropKind::Variable(DataType::User), true),
            prop("role", PropKind::Variable(DataType::Role), true),
        ],
    )
}

fn compare() -> NodeType {
    condition(
        "core.compare",
        vec![
            text("left", true),
            prop("operator", PropKind::Choice(Choices::Comparison), true),
            prop("right", PropKind::Text { templates: true }, false),
        ],
    )
}

/// A moderation action (D-073): it continues at "then", or at "error" if Discord refused
/// (D-082). Each takes the user it acts on and a reason for the audit log.
fn moderation(id: &str, permission: &str, mut props: Vec<Prop>) -> NodeType {
    props.insert(0, prop("user", PropKind::Variable(DataType::User), true));
    props.push(prop("reason", PropKind::Text { templates: true }, false));
    NodeType {
        id: id.to_owned(),
        category: Category::Action,
        pins: vec![
            flow("run", Direction::In),
            flow("then", Direction::Out),
            flow("error", Direction::Out),
        ],
        props,
        provides: Vec::new(),
        intents: Vec::new(),
        permission: Some(permission.to_owned()),
    }
}

fn ban() -> NodeType {
    moderation(
        "core.ban",
        "ban_members",
        vec![prop(
            "delete_messages",
            PropKind::Choice(Choices::DeleteMessages),
            true,
        )],
    )
}

fn kick() -> NodeType {
    moderation("core.kick", "kick_members", Vec::new())
}

fn timeout() -> NodeType {
    moderation(
        "core.timeout",
        "moderate_members",
        // Discord's longest timeout: 28 days.
        vec![prop("minutes", PropKind::Number { max: 40_320 }, true)],
    )
}

fn role_action(id: &str) -> NodeType {
    moderation(
        id,
        "manage_roles",
        vec![prop("role", PropKind::Variable(DataType::Role), true)],
    )
}

fn prop(name: &str, kind: PropKind, required: bool) -> Prop {
    Prop {
        name: name.to_owned(),
        kind,
        required,
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
