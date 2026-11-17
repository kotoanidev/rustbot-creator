//! Graph validation against the node registry (D-047, D-052–D-054).

use rustbot_core::registry::{
    Category, DataType, Direction, NodeType, Pin, PinKind, ProblemKind, RegisterError,
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
    assert_eq!(
        kinds(&ping("   ")),
        vec![ProblemKind::MissingProp("content".to_owned())]
    );
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
