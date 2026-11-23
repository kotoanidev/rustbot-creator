//! The built-in `core.` node types.

use crate::registry::{
    Category, Choices, DataType, Direction, NodeType, Pin, PinKind, Prop, PropKind, Variable,
};

pub(crate) fn all() -> Vec<NodeType> {
    vec![
        slash_command(),
        event("core.member_join", &["user", "server"], &["GUILD_MEMBERS"]),
        event("core.member_leave", &["user", "server"], &["GUILD_MEMBERS"]),
        message_event(),
        event("core.ready", &[], &[]),
        event(
            "core.reaction_add",
            &["user", "channel", "server", "emoji"],
            &["GUILD_MESSAGE_REACTIONS"],
        ),
        event(
            "core.reaction_remove",
            &["user", "channel", "server", "emoji"],
            &["GUILD_MESSAGE_REACTIONS"],
        ),
        reply(),
        send_message(),
        delete_message(),
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
        triggers: Vec::new(),
    }
}

/// An event trigger (D-091): its graph runs when the event comes, with `provides` filled
/// in from it. `emoji` and `message` are text; the others have the usual fields.
fn event(id: &str, provides: &[&str], intents: &[&str]) -> NodeType {
    let provides = provides
        .iter()
        .map(|&name| {
            let ty = match name {
                "user" => DataType::User,
                "channel" => DataType::Channel,
                "server" => DataType::Server,
                _ => DataType::Text,
            };
            var(name, ty)
        })
        .collect();
    NodeType {
        id: id.to_owned(),
        category: Category::Trigger,
        pins: vec![flow("then", Direction::Out)],
        props: Vec::new(),
        provides,
        intents: intents.iter().map(|&i| i.to_owned()).collect(),
        permission: None,
        triggers: Vec::new(),
    }
}

/// "New message": skips bots unless "Also messages from bots" is on, and always the bot
/// itself (D-093). Reading the text needs the Message Content intent (D-095).
fn message_event() -> NodeType {
    let mut node = event(
        "core.message",
        &["user", "channel", "server", "message"],
        &["GUILD_MESSAGES", "MESSAGE_CONTENT"],
    );
    node.props = vec![prop("include_bots", PropKind::Bool, false)];
    node
}

/// Deletes the message of a "New message" graph (D-094).
fn delete_message() -> NodeType {
    NodeType {
        id: "core.delete_message".to_owned(),
        category: Category::Action,
        pins: vec![flow("run", Direction::In), flow("then", Direction::Out)],
        props: Vec::new(),
        provides: Vec::new(),
        intents: Vec::new(),
        permission: None,
        triggers: vec!["core.message".to_owned()],
    }
}

fn reply() -> NodeType {
    NodeType {
        id: "core.reply".to_owned(),
        category: Category::Action,
        pins: vec![flow("run", Direction::In), flow("then", Direction::Out)],
        props: message(vec![prop("private", PropKind::Bool, false)]),
        provides: Vec::new(),
        intents: Vec::new(),
        permission: None,
        // A command's reply, or a reply to the message (D-094).
        triggers: vec!["core.slash_command".to_owned(), "core.message".to_owned()],
    }
}

/// Sends a message to a channel (D-073): one of the command's, an option, or an ID (D-088).
fn send_message() -> NodeType {
    let mut props = vec![prop("channel", PropKind::Variable(DataType::Channel), true)];
    props.extend(message(Vec::new()));
    NodeType {
        id: "core.send_message".to_owned(),
        category: Category::Action,
        pins: vec![flow("run", Direction::In), flow("then", Direction::Out)],
        props,
        provides: Vec::new(),
        intents: Vec::new(),
        permission: None,
        triggers: Vec::new(),
    }
}

/// A message's fields: its text, `extra`, then the embed (D-079, D-087). The form groups
/// the `embed_…` fields into a section, and the message needs text or an embed.
fn message(extra: Vec<Prop>) -> Vec<Prop> {
    let template = |name| prop(name, PropKind::Text { templates: true }, false);
    let mut props = vec![template("content")];
    props.extend(extra);
    props.extend([
        template("embed_title"),
        template("embed_description"),
        prop("embed_color", PropKind::Color, false),
        template("embed_image"),
        template("embed_footer"),
    ]);
    props
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
        triggers: Vec::new(),
    }
}

fn has_permission() -> NodeType {
    let mut node = condition(
        "core.has_permission",
        vec![
            prop("user", PropKind::Variable(DataType::User), true),
            prop("permission", PropKind::Choice(Choices::Permission), true),
        ],
    );
    // Discord sends a member's permissions with commands only.
    node.triggers = vec!["core.slash_command".to_owned()];
    node
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
        triggers: Vec::new(),
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
