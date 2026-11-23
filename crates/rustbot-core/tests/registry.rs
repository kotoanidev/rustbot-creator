//! Graph validation against the node registry (D-047, D-052–D-054).

use rustbot_core::registry::{
    Category, ComponentProblem, DataType, Direction, NodeType, OptionProblem, Pin, PinKind,
    ProblemKind, PropKind, RegisterError,
};
use rustbot_core::{Graph, NodeId, Registry};

/// A graph from JSON in the D-051 format.
fn graph(json: &str) -> Graph {
    serde_json::from_str(json).unwrap()
}

/// The `/ping` command of D-051 with a different reply.
fn ping(content: &str) -> Graph {
    graph(&format!(
        r#"{{ "schema": 1,
              "nodes": {{
                "n1": {{ "type": "core.slash_command",
                        "props": {{ "name": "ping", "description": "Replies with Pong!" }} }},
                "n2": {{ "type": "core.reply", "props": {{ "content": {} }} }} }},
              "edges": [ {{ "from": "n1.then", "to": "n2.run" }} ] }}"#,
        serde_json::to_string(content).unwrap()
    ))
}

fn kinds(graph: &Graph) -> Vec<ProblemKind> {
    Registry::with_core()
        .validate(graph)
        .into_iter()
        .map(|p| p.kind)
        .collect()
}

#[test]
fn core_nodes_register() {
    let mut registry = Registry::default();
    for node_type in Registry::with_core().iter() {
        registry.register(node_type.clone()).unwrap();
    }
    assert!(registry.get("core.slash_command").is_some());
    assert!(registry.get("core.reply").is_some());
}

#[test]
fn the_ping_graph_is_valid() {
    assert_eq!(kinds(&ping("Pong, {user.name}!")), vec![]);
    assert_eq!(
        kinds(&ping("{{literal}} {user.mention} in {channel.name}")),
        vec![]
    );
}

#[test]
fn templates_are_checked() {
    let unknown = |path: &str| ProblemKind::UnknownVariable {
        prop: "content".to_owned(),
        path: path.to_owned(),
    };
    assert_eq!(kinds(&ping("{usr.name}")), vec![unknown("usr.name")]);
    assert_eq!(kinds(&ping("{user.nmae}")), vec![unknown("user.nmae")]);
    // An object needs a field.
    assert_eq!(kinds(&ping("{user}")), vec![unknown("user")]);
    assert!(matches!(
        kinds(&ping("{user.name")).as_slice(),
        [ProblemKind::BadTemplate { .. }]
    ));
}

#[test]
fn required_props() {
    assert_eq!(kinds(&ping("   ")), vec![ProblemKind::EmptyMessage]);
    let g = graph(
        r#"{ "schema": 1, "nodes": {
              "n1": { "type": "core.slash_command", "props": { "name": "ping", "description": 3, "colour": "x" } } } }"#,
    );
    assert_eq!(
        kinds(&g),
        vec![
            ProblemKind::WrongPropType("description".to_owned()),
            ProblemKind::UnknownProp("colour".to_owned()),
        ]
    );
}

#[test]
fn exactly_one_trigger() {
    let none = graph(
        r#"{ "schema": 1, "nodes": { "n1": { "type": "core.reply", "props": { "content": "hi" } } } }"#,
    );
    assert_eq!(kinds(&none), vec![ProblemKind::NoTrigger]);

    let mut two = ping("hi");
    let trigger = two.nodes[&NodeId("n1".to_owned())].clone();
    two.nodes.insert(NodeId("n3".to_owned()), trigger);
    assert_eq!(kinds(&two), vec![ProblemKind::SeveralTriggers]);
}

#[test]
fn unknown_node_types_are_reported_once() {
    let mut g = ping("hi");
    g.nodes.get_mut(&NodeId("n2".to_owned())).unwrap().kind = "someplugin.dance".to_owned();
    assert_eq!(
        kinds(&g),
        vec![ProblemKind::UnknownNodeType("someplugin.dance".to_owned())]
    );
}

#[test]
fn wires_are_checked() {
    let wired = |from: &str, to: &str| {
        let mut g = ping("hi");
        g.edges[0].from = from.parse().unwrap();
        g.edges[0].to = to.parse().unwrap();
        kinds(&g)
    };
    assert_eq!(
        wired("n1.then", "n2.nope"),
        vec![ProblemKind::UnknownPin("n2.nope".parse().unwrap())]
    );
    assert_eq!(
        wired("n9.then", "n2.run"),
        vec![ProblemKind::UnknownPin("n9.then".parse().unwrap())]
    );
    assert_eq!(
        wired("n2.run", "n1.then"),
        vec![ProblemKind::WrongDirection("n2.run".parse().unwrap())]
    );
}

#[test]
fn flow_cannot_feed_data() {
    let mut registry = Registry::with_core();
    registry
        .register(NodeType {
            id: "test.shout".to_owned(),
            category: Category::Action,
            pins: vec![
                Pin {
                    name: "run".to_owned(),
                    direction: Direction::In,
                    kind: PinKind::Flow,
                },
                Pin {
                    name: "text".to_owned(),
                    direction: Direction::In,
                    kind: PinKind::Data(DataType::Text),
                },
            ],
            props: vec![],
            provides: vec![],
            intents: vec![],
            permission: None,
            triggers: vec![],
        })
        .unwrap();
    let mut g = ping("hi");
    g.nodes.insert(
        NodeId("n3".to_owned()),
        serde_json::from_str(r#"{ "type": "test.shout" }"#).unwrap(),
    );
    g.edges[0].to = "n3.text".parse().unwrap();
    let problems: Vec<_> = registry.validate(&g).into_iter().map(|p| p.kind).collect();
    assert_eq!(
        problems,
        vec![ProblemKind::KindMismatch {
            from: "n1.then".parse().unwrap(),
            to: "n3.text".parse().unwrap(),
        }]
    );
}

#[test]
fn register_refuses_bad_ids() {
    let mut registry = Registry::with_core();
    let mut node_type = registry.get("core.reply").unwrap().clone();
    assert_eq!(
        registry.register(node_type.clone()),
        Err(RegisterError::Duplicate("core.reply".to_owned()))
    );
    node_type.id = "reply".to_owned();
    assert_eq!(
        registry.register(node_type),
        Err(RegisterError::InvalidId("reply".to_owned()))
    );
}

/// `/ban` with options (D-076) and a reply that uses them.
fn ban(options: &str, content: &str) -> Graph {
    graph(&format!(
        r#"{{ "schema": 1,
              "nodes": {{
                "n1": {{ "type": "core.slash_command",
                        "props": {{ "name": "ban", "description": "Bans", "options": {options} }} }},
                "n2": {{ "type": "core.reply", "props": {{ "content": {} }} }} }},
              "edges": [ {{ "from": "n1.then", "to": "n2.run" }} ] }}"#,
        serde_json::to_string(content).unwrap()
    ))
}

#[test]
fn command_options_become_variables() {
    let options = r#"[
        { "name": "cel", "type": "user", "description": "Who", "required": true },
        { "name": "powód", "type": "text", "description": "Why" },
        { "name": "rola", "type": "role", "description": "Which role" }
    ]"#;
    let ok = ban(
        options,
        "{option.cel.mention} {option.powód} {option.rola.name}",
    );
    assert_eq!(kinds(&ok), Vec::new());
    // A user needs a field, plain text has none, and unknown options are reported.
    for path in ["option.cel", "option.powód.name", "option.nope", "option"] {
        let bad = ban(options, &format!("{{{path}}}"));
        assert_eq!(
            kinds(&bad),
            vec![ProblemKind::UnknownVariable {
                prop: "content".to_owned(),
                path: path.to_owned(),
            }],
            "{path}"
        );
    }
    let scope = Registry::with_core().scope(&ok);
    assert!(
        scope
            .iter()
            .any(|v| v.name == "option.rola" && v.ty == DataType::Role)
    );
    assert!(
        scope
            .iter()
            .any(|v| v.name == "user" && v.ty == DataType::User)
    );
}

#[test]
fn command_options_follow_discords_rules() {
    let problems = |options: &str| -> Vec<(usize, OptionProblem)> {
        kinds(&ban(options, "ok"))
            .into_iter()
            .map(|kind| match kind {
                ProblemKind::BadOption { option, problem } => (option, problem),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    };
    assert_eq!(
        problems(
            r#"[ { "name": "Cel", "type": "user", "description": "x" },
                 { "name": "a b", "type": "text", "description": "x" },
                 { "name": "ok", "type": "text", "description": " " },
                 { "name": "ok", "type": "server", "description": "x", "required": true } ]"#
        ),
        vec![
            (0, OptionProblem::Name),
            (1, OptionProblem::Name),
            (2, OptionProblem::Description),
            (3, OptionProblem::Duplicate),
            (3, OptionProblem::Type),
            (3, OptionProblem::RequiredAfterOptional),
        ]
    );
    let many: Vec<String> = (0..26)
        .map(|i| format!(r#"{{ "name": "o{i}", "type": "text", "description": "x" }}"#))
        .collect();
    assert_eq!(
        problems(&format!("[{}]", many.join(","))),
        vec![(25, OptionProblem::TooMany)]
    );
    assert_eq!(
        kinds(&ban(r#""not a list""#, "ok")),
        vec![ProblemKind::WrongPropType("options".to_owned())]
    );
}

/// `/ban` with a user, a text and a role option, followed by `condition`.
fn with_condition(condition: &serde_json::Value) -> Graph {
    serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command", "props": {
                "name": "ban", "description": "Bans", "options": [
                    { "name": "cel", "type": "user", "description": "Who" },
                    { "name": "powód", "type": "text", "description": "Why" },
                    { "name": "rola", "type": "role", "description": "Role" } ] } },
            "n2": condition
        },
        "edges": [ { "from": "n1.then", "to": "n2.run" } ]
    }))
    .unwrap()
}

#[test]
fn conditions_take_typed_variables_ids_and_choices() {
    let ok = [
        serde_json::json!({ "type": "core.has_permission",
            "props": { "user": "user", "permission": "ban_members" } }),
        serde_json::json!({ "type": "core.has_permission",
            "props": { "user": "option.cel", "permission": "administrator" } }),
        serde_json::json!({ "type": "core.has_role",
            "props": { "user": "option.cel", "role": "option.rola" } }),
        serde_json::json!({ "type": "core.has_role",
            "props": { "user": "user", "role": "123456789012345678" } }),
        serde_json::json!({ "type": "core.compare",
            "props": { "left": "{option.powód}", "operator": "contains", "right": "" } }),
    ];
    for node in ok {
        assert_eq!(kinds(&with_condition(&node)), Vec::new(), "{node}");
    }
    let wrong = |props: serde_json::Value| kinds(&with_condition(&props));
    assert_eq!(
        wrong(serde_json::json!({ "type": "core.has_role",
            "props": { "user": "option.powód", "role": "12345" } })),
        vec![
            ProblemKind::WrongVariableType {
                prop: "user".to_owned(),
                path: "option.powód".to_owned()
            },
            ProblemKind::UnknownVariable {
                prop: "role".to_owned(),
                path: "12345".to_owned()
            },
        ]
    );
    assert_eq!(
        wrong(serde_json::json!({ "type": "core.has_permission",
            "props": { "user": "", "permission": "fly" } })),
        vec![
            ProblemKind::MissingProp("user".to_owned()),
            ProblemKind::WrongPropType("permission".to_owned()),
        ]
    );
    // A user field never takes an ID, only a role field does.
    assert_eq!(
        wrong(serde_json::json!({ "type": "core.has_permission",
            "props": { "user": "123456789012345678", "permission": "ban_members" } })),
        vec![ProblemKind::UnknownVariable {
            prop: "user".to_owned(),
            path: "123456789012345678".to_owned()
        }]
    );
}

#[test]
fn new_nodes_start_with_values_of_the_right_kind() {
    for node_type in Registry::with_core().iter() {
        for prop in &node_type.props {
            let value = prop.kind.default_value();
            let expected_list = matches!(
                prop.kind,
                PropKind::Options
                    | PropKind::Buttons
                    | PropKind::MenuOptions
                    | PropKind::FormFields
            );
            assert_eq!(
                value.is_array(),
                expected_list,
                "{} {}",
                node_type.id,
                prop.name
            );
        }
    }
}

#[test]
fn moderation_takes_a_user_a_role_and_minutes() {
    let ok = [
        serde_json::json!({ "type": "core.ban", "props": {
            "user": "option.cel", "delete_messages": "day", "reason": "{option.powód}" } }),
        serde_json::json!({ "type": "core.kick", "props": { "user": "option.cel" } }),
        serde_json::json!({ "type": "core.timeout", "props": {
            "user": "option.cel", "minutes": "40320", "reason": "" } }),
        serde_json::json!({ "type": "core.timeout", "props": {
            "user": "option.cel", "minutes": " 1.5 " } }),
        serde_json::json!({ "type": "core.timeout", "props": {
            "user": "option.cel", "minutes": "{option.powód}" } }),
        serde_json::json!({ "type": "core.add_role", "props": {
            "user": "user", "role": "option.rola" } }),
        serde_json::json!({ "type": "core.remove_role", "props": {
            "user": "option.cel", "role": "123456789012345678" } }),
    ];
    for node in ok {
        assert_eq!(kinds(&with_condition(&node)), Vec::new(), "{node}");
    }
    let bad_number = |minutes: &str| {
        kinds(&with_condition(
            &serde_json::json!({ "type": "core.timeout",
            "props": { "user": "option.cel", "minutes": minutes } }),
        ))
    };
    for minutes in ["0", "-5", "40321", "ten", "NaN", "inf"] {
        assert_eq!(
            bad_number(minutes),
            vec![ProblemKind::BadNumber {
                prop: "minutes".to_owned(),
                max: 40_320
            }],
            "{minutes}"
        );
    }
    assert_eq!(
        bad_number(""),
        vec![ProblemKind::MissingProp("minutes".to_owned())]
    );
    assert_eq!(
        bad_number("{option.nope}"),
        vec![ProblemKind::UnknownVariable {
            prop: "minutes".to_owned(),
            path: "option.nope".to_owned()
        }]
    );
}

/// `/ban` wired as `edges`, with `n2` the node under test and `n3` a "Has permission?".
fn warnings_of(node: &serde_json::Value, edges: &serde_json::Value) -> Vec<ProblemKind> {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command", "props": {
                "name": "ban", "description": "Bans", "options": [
                    { "name": "cel", "type": "user", "description": "Who" },
                    { "name": "rola", "type": "role", "description": "Role" } ] } },
            "n2": node,
            "n3": { "type": "core.has_permission",
                    "props": { "user": "user", "permission": "ban_members" } },
            "n4": { "type": "core.has_permission",
                    "props": { "user": "option.cel", "permission": "ban_members" } }
        },
        "edges": edges
    }))
    .unwrap();
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&graph), Vec::new());
    registry
        .warnings(&graph)
        .into_iter()
        .map(|w| w.kind)
        .filter(|kind| *kind != ProblemKind::NoReply)
        .collect()
}

#[test]
fn actions_on_others_need_a_permission_check_on_the_author() {
    let ban = serde_json::json!({ "type": "core.ban",
        "props": { "user": "option.cel", "delete_messages": "none" } });
    let unchecked = vec![ProblemKind::NoPermissionCheck {
        permission: "ban_members".to_owned(),
    }];
    let direct = serde_json::json!([{ "from": "n1.then", "to": "n2.run" }]);
    let behind = |pin: &str, check: &str| {
        serde_json::json!([
            { "from": "n1.then", "to": format!("{check}.run") },
            { "from": format!("{check}.{pin}"), "to": "n2.run" }
        ])
    };
    assert_eq!(warnings_of(&ban, &direct), unchecked);
    assert_eq!(warnings_of(&ban, &behind("yes", "n3")), Vec::new());
    assert_eq!(warnings_of(&ban, &behind("no", "n3")), unchecked);
    // Checking the target instead of the author guards nothing.
    assert_eq!(warnings_of(&ban, &behind("yes", "n4")), unchecked);
    // A join: one path is checked, the other is not.
    let join = serde_json::json!([
        { "from": "n1.then", "to": "n3.run" },
        { "from": "n3.yes", "to": "n2.run" },
        { "from": "n3.no", "to": "n2.run" }
    ]);
    assert_eq!(warnings_of(&ban, &join), unchecked);

    // The author acting on themselves needs nothing, unless they pick the role.
    let fixed = serde_json::json!({ "type": "core.add_role",
        "props": { "user": "user", "role": "123456789012345678" } });
    assert_eq!(warnings_of(&fixed, &direct), Vec::new());
    let picked = serde_json::json!({ "type": "core.add_role",
        "props": { "user": "user", "role": "option.rola" } });
    assert_eq!(
        warnings_of(&picked, &direct),
        vec![ProblemKind::NoPermissionCheck {
            permission: "manage_roles".to_owned()
        }]
    );
    let self_kick = serde_json::json!({ "type": "core.kick", "props": { "user": "user" } });
    assert_eq!(warnings_of(&self_kick, &direct), Vec::new());
}

/// The warnings of `/say` with these nodes and edges; `n1` is the command.
fn reply_warnings(nodes: &serde_json::Value, edges: &serde_json::Value) -> Vec<ProblemKind> {
    let mut nodes = nodes.clone();
    nodes["n1"] = serde_json::json!({ "type": "core.slash_command", "props": {
        "name": "say", "description": "Says", "options": [
            { "name": "kanał", "type": "channel", "description": "Where" } ] } });
    let graph: Graph =
        serde_json::from_value(serde_json::json!({ "schema": 1, "nodes": nodes, "edges": edges }))
            .unwrap();
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&graph), Vec::new());
    registry
        .warnings(&graph)
        .into_iter()
        .map(|w| w.kind)
        .collect()
}

#[test]
fn a_command_warns_when_a_path_ends_without_a_reply() {
    let nodes = serde_json::json!({
        "n2": { "type": "core.send_message", "props": {
            "channel": "option.kanał", "content": "", "embed_title": "News" } },
        "n3": { "type": "core.reply", "props": { "content": "Sent", "private": true } },
        "n4": { "type": "core.has_permission",
                "props": { "user": "user", "permission": "manage_messages" } },
        "n6": { "type": "core.send_message", "props": {
            "channel": "123456789012345678", "content": "Hi" } }
    });
    let edges = |list: &[(&str, &str)]| {
        serde_json::Value::Array(
            list.iter()
                .map(|(from, to)| serde_json::json!({ "from": from, "to": to }))
                .collect(),
        )
    };
    let warn = vec![ProblemKind::NoReply];
    let none: Vec<ProblemKind> = Vec::new();
    assert_eq!(reply_warnings(&nodes, &edges(&[])), warn);
    assert_eq!(
        reply_warnings(&nodes, &edges(&[("n1.then", "n2.run")])),
        warn
    );
    let sent = [("n1.then", "n2.run"), ("n2.then", "n3.run")];
    assert_eq!(reply_warnings(&nodes, &edges(&sent)), none);
    // A reply in a sibling that runs earlier counts too.
    let siblings = [("n1.then", "n3.run"), ("n1.then", "n6.run")];
    assert_eq!(reply_warnings(&nodes, &edges(&siblings)), none);
    // Both branches of a condition must reply.
    let one_branch = [("n1.then", "n4.run"), ("n4.yes", "n3.run")];
    assert_eq!(reply_warnings(&nodes, &edges(&one_branch)), warn);
    let both = [
        ("n1.then", "n4.run"),
        ("n4.yes", "n3.run"),
        ("n4.no", "n6.run"),
        ("n6.then", "n3.run"),
    ];
    assert_eq!(reply_warnings(&nodes, &edges(&both)), none);
}

#[test]
fn messages_need_text_or_an_embed_and_colours_are_hex() {
    let check = |props: serde_json::Value| {
        kinds(&with_condition(
            &serde_json::json!({ "type": "core.send_message", "props": props }),
        ))
    };
    assert_eq!(
        check(serde_json::json!({ "channel": "channel", "content": " " })),
        vec![ProblemKind::EmptyMessage]
    );
    for ok in [
        serde_json::json!({ "channel": "channel", "embed_description": "{option.powód}" }),
        serde_json::json!({ "channel": "123456789012345678", "content": "x",
            "embed_color": "#5865f2" }),
        serde_json::json!({ "channel": "channel", "content": "x", "embed_color": "{option.powód}" }),
    ] {
        assert_eq!(check(ok.clone()), Vec::new(), "{ok}");
    }
    for color in ["5865F2", "#5865F", "#GGGGGG", "red"] {
        assert_eq!(
            check(
                serde_json::json!({ "channel": "channel", "content": "x", "embed_color": color })
            ),
            vec![ProblemKind::BadColor("embed_color".to_owned())],
            "{color}"
        );
    }
    // A channel field takes an ID, a user field does not (D-088).
    assert_eq!(
        check(serde_json::json!({ "channel": "option.cel", "content": "x" })),
        vec![ProblemKind::WrongVariableType {
            prop: "channel".to_owned(),
            path: "option.cel".to_owned()
        }]
    );
}

/// 40 conditions whose "yes" and "no" both lead to the next reach the last one 2^40 ways;
/// the warnings still come at once, as the editor draws them (D-090).
#[test]
fn warnings_are_quick_on_many_joins() {
    let mut nodes = serde_json::Map::new();
    let mut edges = vec![serde_json::json!({ "from": "n1.then", "to": "n2.run" })];
    nodes.insert(
        "n1".to_owned(),
        serde_json::json!({ "type": "core.slash_command",
        "props": { "name": "deep", "description": "Deep" } }),
    );
    for i in 2..42 {
        nodes.insert(
            format!("n{i}"),
            serde_json::json!({ "type": "core.compare",
            "props": { "left": "a", "operator": "equals", "right": "b" } }),
        );
        for pin in ["yes", "no"] {
            edges.push(serde_json::json!({ "from": format!("n{i}.{pin}"), "to": format!("n{}.run", i + 1) }));
        }
    }
    nodes.insert(
        "n42".to_owned(),
        serde_json::json!({ "type": "core.reply",
        "props": { "content": "Deep" } }),
    );
    let graph: Graph =
        serde_json::from_value(serde_json::json!({ "schema": 1, "nodes": nodes, "edges": edges }))
            .unwrap();
    let started = std::time::Instant::now();
    assert_eq!(Registry::with_core().warnings(&graph), Vec::new());
    assert!(
        started.elapsed().as_millis() < 100,
        "{:?}",
        started.elapsed()
    );
}

/// A graph started by `trigger`, followed by `node`.
fn event_graph(trigger: &str, node: &serde_json::Value) -> Vec<ProblemKind> {
    kinds(
        &serde_json::from_value(serde_json::json!({
            "schema": 1,
            "nodes": { "n1": { "type": trigger }, "n2": node },
            "edges": [ { "from": "n1.then", "to": "n2.run" } ]
        }))
        .unwrap(),
    )
}

#[test]
fn events_offer_their_variables_and_allow_their_nodes() {
    let reply = |content: &str| serde_json::json!({ "type": "core.reply", "props": { "content": content } });
    let send = |content: &str| {
        serde_json::json!({ "type": "core.send_message",
            "props": { "channel": "123456789012345678", "content": content } })
    };
    let none: Vec<ProblemKind> = Vec::new();
    assert_eq!(
        event_graph("core.message", &reply("You said {message}, {user.mention}")),
        none
    );
    assert_eq!(
        event_graph(
            "core.message",
            &serde_json::json!({ "type": "core.delete_message" })
        ),
        none
    );
    assert_eq!(
        event_graph(
            "core.member_join",
            &send("Welcome {user.mention} to {server.name}")
        ),
        none
    );
    assert_eq!(
        event_graph("core.member_leave", &send("Bye {user.name}")),
        none
    );
    assert_eq!(
        event_graph(
            "core.reaction_add",
            &send("{user.name} reacted {emoji} in {channel.mention}")
        ),
        none
    );
    assert_eq!(event_graph("core.ready", &send("Online")), none);

    let commands_only = ProblemKind::WrongTrigger {
        allowed: vec![
            "core.slash_command".to_owned(),
            "core.message".to_owned(),
            "core.click".to_owned(),
        ],
    };
    assert_eq!(
        event_graph("core.member_join", &reply("Hi")),
        vec![commands_only]
    );
    assert_eq!(
        event_graph(
            "core.message",
            &serde_json::json!({ "type": "core.has_permission",
            "props": { "user": "user", "permission": "administrator" } })
        ),
        vec![ProblemKind::WrongTrigger {
            allowed: vec!["core.slash_command".to_owned(), "core.click".to_owned()]
        }]
    );
    assert_eq!(
        event_graph(
            "core.message",
            &serde_json::json!({ "type": "core.reply",
            "props": { "content": "Psst", "private": true } })
        ),
        vec![ProblemKind::PrivateOutsideCommand]
    );
    // Variables come from the event: no message text when a member joins.
    assert_eq!(
        event_graph("core.member_join", &send("{message}")),
        vec![ProblemKind::UnknownVariable {
            prop: "content".to_owned(),
            path: "message".to_owned()
        }]
    );
    assert_eq!(
        event_graph("core.ready", &send("{user.name}")),
        vec![ProblemKind::UnknownVariable {
            prop: "content".to_owned(),
            path: "user.name".to_owned()
        }]
    );
}

/// A command whose reply carries `buttons` and `menu`, then `after` wired as given.
fn clicks(
    message: &serde_json::Value,
    after: &serde_json::Value,
    edges: &serde_json::Value,
) -> Graph {
    let mut nodes = after.clone();
    nodes["n1"] = serde_json::json!({ "type": "core.slash_command", "props": {
        "name": "ask", "description": "Asks", "options": [
            { "name": "target", "type": "user", "description": "Who" } ] } });
    nodes["n2"] = message.clone();
    let mut edges = edges.as_array().unwrap().clone();
    edges.push(serde_json::json!({ "from": "n1.then", "to": "n2.run" }));
    serde_json::from_value(serde_json::json!({ "schema": 1, "nodes": nodes, "edges": edges }))
        .unwrap()
}

#[test]
fn buttons_and_options_add_outputs() {
    let registry = Registry::with_core();
    let node: rustbot_core::Node = serde_json::from_value(serde_json::json!({
        "type": "core.reply", "props": { "content": "Sure?", "buttons": [
            { "id": "b1", "label": "Yes", "style": "success" },
            { "id": "b3", "label": "Docs", "style": "link", "url": "https://example.com" } ],
        "menu": [ { "id": "o2", "label": "Red" } ] } }))
    .unwrap();
    let outputs = |node: &rustbot_core::Node| -> Vec<String> {
        registry
            .pins(node)
            .into_iter()
            .filter(|pin| pin.direction == Direction::Out)
            .map(|pin| pin.name)
            .collect()
    };
    // A link has no output.
    assert_eq!(outputs(&node), ["then", "b1", "o2"]);
    let mut waiting = node.clone();
    waiting.props.insert("wait".to_owned(), "5".into());
    assert_eq!(outputs(&waiting), ["then", "b1", "o2", "timeout"]);
    // Only message nodes have them.
    let mut kick = node;
    kick.kind = "core.kick".to_owned();
    assert_eq!(outputs(&kick), ["then", "error"]);
}

#[test]
fn buttons_and_options_follow_discords_rules() {
    let bad = |prop: &str, item, problem| ProblemKind::BadComponent {
        prop: prop.to_owned(),
        item,
        problem,
    };
    let message = serde_json::json!({ "type": "core.reply", "props": { "content": "Hi",
        "buttons": [
            { "id": "b1", "label": "", "style": "primary" },
            { "id": "b1", "label": "Same", "style": "danger" },
            { "id": "x", "label": "Odd", "style": "secondary" },
            { "id": "b4", "label": "Docs", "style": "link", "url": "javascript:alert(1)" },
            { "id": "b5", "label": "Ok", "style": "success" },
            { "id": "b6", "label": "Sixth", "style": "success" } ],
        "menu": [ { "id": "o1", "label": "x".repeat(101) } ] } });
    let graph = clicks(&message, &serde_json::json!({}), &serde_json::json!([]));
    assert_eq!(
        kinds(&graph),
        vec![
            bad("buttons", 0, ComponentProblem::Label),
            bad("buttons", 1, ComponentProblem::Id),
            bad("buttons", 2, ComponentProblem::Id),
            bad("buttons", 5, ComponentProblem::TooMany),
            bad("buttons", 3, ComponentProblem::Url),
            bad("menu", 0, ComponentProblem::Label),
        ]
    );
}

#[test]
fn after_a_click_that_always_works_only_the_click_is_known() {
    let ask = |wait: &str| {
        serde_json::json!({ "type": "core.reply", "props": { "content": "Sure?", "wait": wait,
            "buttons": [ { "id": "b1", "label": "Yes", "style": "danger" } ] } })
    };
    let after = serde_json::json!({
        "n3": { "type": "core.ban", "props": { "user": "option.target",
            "delete_messages": "none" } },
        "n4": { "type": "core.reply", "props": { "content": "{user.mention} in {channel.mention}",
            "private": true } },
        "n5": { "type": "core.has_permission", "props": { "user": "user",
            "permission": "ban_members" } }
    });
    let edges = serde_json::json!([
        { "from": "n2.b1", "to": "n5.run" },
        { "from": "n5.yes", "to": "n3.run" },
        { "from": "n3.then", "to": "n4.run" }
    ]);
    // Waited for, the command's options stay (D-097).
    assert_eq!(kinds(&clicks(&ask("5"), &after, &edges)), Vec::new());
    // Always working, they are gone; the clicker, channel and server stay.
    let graph = clicks(&ask(""), &after, &edges);
    assert_eq!(
        kinds(&graph),
        vec![ProblemKind::UnknownVariable {
            prop: "user".to_owned(),
            path: "option.target".to_owned()
        }]
    );
    let registry = Registry::with_core();
    let names = |node: &str| -> Vec<String> {
        registry
            .scope_at(&graph, &NodeId(node.to_owned()))
            .into_iter()
            .map(|var| var.name)
            .collect()
    };
    assert_eq!(names("n3"), ["user", "channel", "server"]);
    assert_eq!(names("n2"), ["user", "channel", "server", "option.target"]);
}

#[test]
fn clicks_let_events_reply_and_need_a_user_to_limit() {
    let event = |trigger: &str, wait: &str, only_author: bool| -> Vec<ProblemKind> {
        kinds(
            &serde_json::from_value(serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": trigger },
                    "n2": { "type": "core.send_message", "props": { "channel": "123456789012345678",
                        "content": "Click", "wait": wait, "only_author": only_author,
                        "menu": [ { "id": "o1", "label": "Me" } ] } },
                    "n3": { "type": "core.reply", "props": { "content": "Done", "private": true } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.o1", "to": "n3.run" }
                ]
            }))
            .unwrap(),
        )
    };
    // "Reply" answers the click, privately too, in either mode.
    assert_eq!(event("core.member_join", "", true), Vec::new());
    assert_eq!(event("core.member_join", "10", true), Vec::new());
    // "Bot is ready" has no user to limit the click to (D-100).
    assert_eq!(
        event("core.ready", "10", true),
        vec![ProblemKind::OnlyAuthorWithoutUser]
    );
    assert_eq!(event("core.ready", "10", false), Vec::new());
}

/// A check of the author does not cover a click anyone may make (D-085, D-100).
#[test]
fn a_waited_click_by_anyone_is_not_covered_by_the_authors_check() {
    let confirm = |only_author: bool| {
        let message = serde_json::json!({ "type": "core.reply", "props": { "content": "Sure?",
            "wait": "5", "only_author": only_author,
            "buttons": [ { "id": "b1", "label": "Yes", "style": "danger" } ] } });
        let after = serde_json::json!({
            "n3": { "type": "core.has_permission", "props": { "user": "user",
                "permission": "ban_members" } },
            "n4": { "type": "core.ban", "props": { "user": "option.target",
                "delete_messages": "none" } }
        });
        let edges = serde_json::json!([
            { "from": "n1.then", "to": "n3.run" },
            { "from": "n3.yes", "to": "n2.run" },
            { "from": "n2.b1", "to": "n4.run" }
        ]);
        let mut graph = clicks(&message, &after, &edges);
        // `clicks` wires the trigger to the message; here the check comes first.
        graph
            .edges
            .retain(|edge| edge.from.to_string() != "n1.then" || edge.to.node.0 == "n3");
        let registry = Registry::with_core();
        assert_eq!(registry.validate(&graph), Vec::new());
        registry
            .warnings(&graph)
            .into_iter()
            .map(|w| w.kind)
            .filter(|kind| *kind != ProblemKind::NoReply)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        confirm(false),
        vec![ProblemKind::AnyoneCanClick {
            permission: "ban_members".to_owned()
        }]
    );
    assert_eq!(confirm(true), Vec::new());
}

/// "The event's message" exists only in "New message" and reactions (D-103).
#[test]
fn message_actions_need_a_message() {
    let react = |message: &str| {
        serde_json::json!({ "type": "core.react",
            "props": { "message": message, "emoji": "👍" } })
    };
    assert_eq!(event_graph("core.message", &react("event")), Vec::new());
    assert_eq!(
        event_graph("core.reaction_add", &react("event")),
        Vec::new()
    );
    assert_eq!(
        event_graph("core.member_join", &react("event")),
        vec![ProblemKind::NoEventMessage]
    );
    assert_eq!(event_graph("core.member_join", &react("last")), Vec::new());
    // A new node starts with the last message sent, which every graph has.
    let registry = Registry::with_core();
    let react = registry.get("core.react").unwrap();
    assert_eq!(react.props[0].kind.default_value(), "last");
}

/// A registry with the memory of a moderation bot (D-106, D-107).
fn with_memory() -> Registry {
    let var = |name: &str, ty, scope| rustbot_core::MemoryVar {
        name: name.to_owned(),
        ty,
        scope,
        initial: String::new(),
    };
    let mut registry = Registry::with_core();
    registry.set_memory(vec![
        var(
            "warnings",
            rustbot_core::MemoryType::Number,
            rustbot_core::MemoryScope::Member,
        ),
        var(
            "log",
            rustbot_core::MemoryType::Text,
            rustbot_core::MemoryScope::Server,
        ),
        var(
            "motd",
            rustbot_core::MemoryType::Text,
            rustbot_core::MemoryScope::Global,
        ),
    ]);
    registry
}

#[test]
fn memory_is_read_and_changed_where_its_owner_is_known() {
    let registry = with_memory();
    let check = |trigger: &serde_json::Value, node: &serde_json::Value| -> Vec<ProblemKind> {
        let graph: Graph = serde_json::from_value(serde_json::json!({
            "schema": 1,
            "nodes": { "n1": trigger, "n2": node },
            "edges": [ { "from": "n1.then", "to": "n2.run" } ]
        }))
        .unwrap();
        registry
            .validate(&graph)
            .into_iter()
            .map(|p| p.kind)
            .collect()
    };
    let warn = serde_json::json!({ "type": "core.slash_command", "props": {
        "name": "warn", "description": "Warns", "options": [
            { "name": "target", "type": "user", "description": "Who", "required": true } ] } });
    let ready = serde_json::json!({ "type": "core.ready" });
    let change = |variable: &str, amount: &str| {
        serde_json::json!({ "type": "core.memory_change", "props": {
            "variable": variable, "user": "option.target", "amount": amount } })
    };
    let reply = |content: &str| serde_json::json!({ "type": "core.reply", "props": { "content": content } });
    assert_eq!(check(&warn, &change("warnings", "1")), Vec::new());
    assert_eq!(
        check(&warn, &change("warnings", "-{memory.warnings}")),
        Vec::new()
    );
    assert_eq!(
        check(
            &warn,
            &reply(
                "{option.target.mention}: {memory.warnings.option.target}, mine {memory.warnings}, {memory.log} {memory.motd}"
            )
        ),
        Vec::new()
    );
    assert_eq!(
        check(&warn, &change("warnings", "lots")),
        vec![ProblemKind::NotANumber("amount".to_owned())]
    );
    assert_eq!(
        check(&warn, &change("log", "1")),
        vec![ProblemKind::NotANumber("variable".to_owned())]
    );
    assert_eq!(
        check(&warn, &change("nope", "1")),
        vec![ProblemKind::UnknownMemory("nope".to_owned())]
    );
    // A global variable has no "for whom".
    assert_eq!(
        check(&warn, &reply("{memory.motd.option.target}")),
        vec![ProblemKind::UnknownVariable {
            prop: "content".to_owned(),
            path: "memory.motd.option.target".to_owned()
        }]
    );
    // "Bot is ready" knows no server and no user.
    let set = |variable: &str| {
        serde_json::json!({ "type": "core.memory_set", "props": {
            "variable": variable, "value": "Hello" } })
    };
    assert_eq!(check(&ready, &set("motd")), Vec::new());
    assert_eq!(
        check(&ready, &set("log")),
        vec![ProblemKind::MemoryNeedsServer]
    );
    assert_eq!(
        check(&ready, &set("warnings")),
        vec![
            ProblemKind::MemoryNeedsServer,
            ProblemKind::MemoryNeedsUser,
            ProblemKind::NotANumber("value".to_owned())
        ]
    );
}

/// A command with a form: its graph from the nodes after the trigger, wired `then`.
fn form_problems(
    registry: &Registry,
    nodes: &serde_json::Value,
    edges: &serde_json::Value,
) -> Vec<(String, ProblemKind)> {
    let mut all = serde_json::json!({
        "n1": { "type": "core.slash_command", "props": {
            "name": "report", "description": "Reports", "options": [
                { "name": "who", "type": "user", "description": "Who" } ] } }
    });
    all.as_object_mut()
        .unwrap()
        .extend(nodes.as_object().unwrap().clone());
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1, "nodes": all, "edges": edges.clone()
    }))
    .unwrap();
    registry
        .validate(&graph)
        .into_iter()
        .map(|p| (p.node.unwrap().0, p.kind))
        .collect()
}

#[test]
fn forms_come_first_and_their_fields_follow_them() {
    let registry = Registry::with_core();
    let form = |fields: serde_json::Value| {
        serde_json::json!({ "type": "core.show_form", "props": {
            "title": "Report", "fields": fields } })
    };
    let reason = serde_json::json!([
        { "name": "reason", "label": "Why?", "long": true, "required": true, "max": "500" }
    ]);
    let reply = |content: &str| serde_json::json!({ "type": "core.reply", "props": { "content": content } });
    let edge = |from: &str, to: &str| serde_json::json!({ "from": from, "to": to });
    let check = serde_json::json!({ "type": "core.has_permission",
        "props": { "user": "user", "permission": "kick_members" } });

    // After a condition, the form's fields are known after "then", not after "timeout".
    assert_eq!(
        form_problems(
            &registry,
            &serde_json::json!({ "n2": check, "n3": form(reason.clone()),
                "n4": reply("{option.who.mention}: {field.reason}"),
                "n5": { "type": "core.send_message", "props": { "channel": "channel",
                    "content": "{field.reason}" } },
                "n6": reply("Too late") }),
            &serde_json::json!([
                edge("n1.then", "n2.run"),
                edge("n2.yes", "n3.run"),
                edge("n3.then", "n4.run"),
                edge("n3.timeout", "n5.run"),
                edge("n5.then", "n6.run")
            ])
        ),
        vec![
            (
                "n5".to_owned(),
                ProblemKind::UnknownVariable {
                    prop: "content".to_owned(),
                    path: "field.reason".to_owned()
                }
            ),
            ("n6".to_owned(), ProblemKind::ReplyAfterClosedForm),
        ]
    );
    // A reply first, then a form: too late; a form after a waited click is fine.
    assert_eq!(
        form_problems(
            &registry,
            &serde_json::json!({ "n2": { "type": "core.reply", "props": { "content": "Sure?",
                    "wait": "1", "buttons": [ { "id": "b1", "label": "Yes", "style": "primary" } ] } },
                "n3": form(reason.clone()), "n4": form(reason.clone()) }),
            &serde_json::json!([
                edge("n1.then", "n2.run"),
                edge("n2.b1", "n3.run"),
                edge("n2.then", "n4.run")
            ])
        ),
        vec![("n4".to_owned(), ProblemKind::FormNotFirst)]
    );
    // A form shares its wire with nothing, which would run before or after it.
    assert_eq!(
        form_problems(
            &registry,
            &serde_json::json!({ "n2": form(reason), "n3": reply("Hi") }),
            &serde_json::json!([edge("n1.then", "n2.run"), edge("n1.then", "n3.run")])
        ),
        vec![("n2".to_owned(), ProblemKind::FormNotFirst)]
    );
    // Discord's rules for the fields.
    let bad = serde_json::json!([
        { "name": "a-b", "label": "One" },
        { "name": "a_b", "label": "" },
        { "name": "c", "label": "Three", "min": "10", "max": "5" },
        { "name": "d", "label": "Four", "max": "0" },
        { "name": "e", "label": "Five", "placeholder": "x".repeat(101) },
        { "name": "f", "label": "Six" }
    ]);
    let problems: Vec<_> = form_problems(
        &registry,
        &serde_json::json!({ "n2": form(bad) }),
        &serde_json::json!([edge("n1.then", "n2.run")]),
    )
    .into_iter()
    .map(|(_, kind)| kind)
    .collect();
    let form = |field, problem| ProblemKind::BadForm { field, problem };
    assert_eq!(
        problems,
        vec![
            form(Some(1), rustbot_core::registry::FormProblem::Name),
            form(Some(1), rustbot_core::registry::FormProblem::Label),
            form(Some(2), rustbot_core::registry::FormProblem::Length),
            form(Some(3), rustbot_core::registry::FormProblem::Length),
            form(Some(4), rustbot_core::registry::FormProblem::Placeholder),
            form(Some(5), rustbot_core::registry::FormProblem::Count),
        ]
    );
}
