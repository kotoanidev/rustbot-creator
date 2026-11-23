//! Bot templates (D-115–D-118): projects to start from, made of the built-in nodes. A
//! template's texts are Fluent messages, written `"$tpl-…"` in its graphs, so the bot
//! speaks the app's language. Channels and roles stay empty for Problems to point at
//! (D-117).

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use rustbot_core::{
    Graph, GraphFile, GraphKind, Layout, Manifest, MemoryScope, MemoryType, MemoryVar, NodeId,
    Project,
};

use crate::i18n::t;

/// A template: its graphs, as `(kind, name, JSON)`, and its memory variables.
pub(crate) struct Template {
    /// Its card's Fluent messages are `template-<id>` and `template-<id>-about`.
    pub(crate) id: &'static str,
    graphs: &'static [(GraphKind, &'static str, &'static str)],
    memory: &'static [(&'static str, MemoryType, MemoryScope)],
}

macro_rules! graph {
    ($kind:ident, $template:literal, $name:literal) => {
        (
            GraphKind::$kind,
            $name,
            include_str!(concat!("../templates/", $template, "/", $name, ".json")),
        )
    };
}

/// Every template, in the order the cards show them; the first is "Empty".
pub(crate) const TEMPLATES: [Template; 5] = [
    Template {
        id: "empty",
        graphs: &[graph!(Command, "empty", "ping")],
        memory: &[],
    },
    Template {
        id: "moderation",
        graphs: &[
            graph!(Command, "moderation", "ban"),
            graph!(Command, "moderation", "kick"),
            graph!(Command, "moderation", "timeout"),
            graph!(Command, "moderation", "warn"),
        ],
        memory: &[("warnings", MemoryType::Number, MemoryScope::Member)],
    },
    Template {
        id: "welcome",
        graphs: &[
            graph!(Event, "welcome", "member_join"),
            graph!(Event, "welcome", "member_leave"),
        ],
        memory: &[],
    },
    Template {
        id: "roles",
        graphs: &[graph!(Command, "roles", "roles")],
        memory: &[],
    },
    Template {
        id: "report",
        graphs: &[graph!(Command, "report", "report")],
        memory: &[],
    },
];

/// A project named after its folder, made from `template`.
pub(crate) fn create(root: &Path, template: &Template) -> Result<Project, rustbot_core::Error> {
    let name = root
        .file_name()
        .map_or_else(|| "bot".to_owned(), |n| n.to_string_lossy().into_owned());
    let mut manifest = Manifest::new(&name);
    manifest.memory = template
        .memory
        .iter()
        .map(|&(name, ty, scope)| MemoryVar {
            name: name.to_owned(),
            ty,
            scope,
            initial: String::new(),
        })
        .collect();
    let mut project = Project::create(root, manifest)?;
    for &(kind, name, json) in template.graphs {
        let graph = graph(json);
        let file = GraphFile {
            kind,
            name: name.to_owned(),
            layout: layout(&graph),
            graph,
        };
        project.save_graph(&file)?;
        project.graphs.push(file);
    }
    Ok(project)
}

/// A template's graph, its `"$tpl-…"` texts in the app's language.
fn graph(json: &str) -> Graph {
    let mut value: serde_json::Value =
        serde_json::from_str(json).expect("a template's graph is JSON");
    translate(&mut value);
    serde_json::from_value(value).expect("a template's graph is a graph")
}

fn translate(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            if let Some(id) = text.strip_prefix('$') {
                *text = t(id);
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(translate),
        serde_json::Value::Object(map) => map.values_mut().for_each(translate),
        _ => {}
    }
}

/// Columns by the distance from the trigger along the wires, and rows in the order the
/// wires reach the nodes, as a person would lay them out.
fn layout(graph: &Graph) -> Layout {
    let starts = graph
        .nodes
        .keys()
        .filter(|id| !graph.edges.iter().any(|edge| &edge.to.node == *id));
    let mut column: BTreeMap<NodeId, usize> = BTreeMap::new();
    let mut order = Vec::new();
    let mut next: VecDeque<NodeId> = starts.cloned().collect();
    for id in &next {
        column.insert(id.clone(), 0);
    }
    while let Some(id) = next.pop_front() {
        order.push(id.clone());
        for edge in graph.edges.iter().filter(|edge| edge.from.node == id) {
            if !column.contains_key(&edge.to.node) {
                column.insert(edge.to.node.clone(), column[&id] + 1);
                next.push_back(edge.to.node.clone());
            }
        }
    }
    let mut rows: BTreeMap<usize, usize> = BTreeMap::new();
    let mut layout = Layout::default();
    for id in order {
        let col = column[&id];
        let row = rows.entry(col).or_default();
        // Columns 300 apart fit a node and its wires; rows 150 apart fit a tall node.
        #[allow(clippy::cast_precision_loss)] // a few dozen nodes
        let position = [80. + 300. * col as f32, 120. + 150. * *row as f32];
        *row += 1;
        layout.nodes.insert(id, position);
    }
    layout
}

#[cfg(test)]
mod tests {
    use rustbot_core::registry::ProblemKind;

    use super::TEMPLATES;

    /// Every template makes a project whose only problems are the channels and roles to
    /// fill in (D-117); filled in, it generates. Its texts exist in both languages.
    #[test]
    fn templates_make_projects_with_only_ids_to_fill_in() {
        let english = include_str!("../locales/en/main.ftl");
        let polish = include_str!("../locales/pl/main.ftl");
        for template in &TEMPLATES {
            for key in [
                format!("template-{}", template.id),
                format!("template-{}-about", template.id),
            ] {
                assert!(english.contains(&format!("\n{key} = ")), "{key}");
            }
            for &(_, name, json) in template.graphs {
                for id in json.split('"').filter_map(|part| part.strip_prefix('$')) {
                    for source in [english, polish] {
                        assert!(source.contains(&format!("\n{id} = ")), "{name}: {id}");
                    }
                }
            }
            let dir = tempfile::tempdir().unwrap();
            let mut project = super::create(&dir.path().join("bot"), template).unwrap();
            let mut registry = rustbot_core::Registry::with_core();
            registry.set_memory(project.manifest.memory.clone());
            for file in &mut project.graphs {
                for problem in registry.validate(&file.graph) {
                    let ProblemKind::MissingProp(prop) = &problem.kind else {
                        panic!("{} {}: {problem:?}", template.id, file.name);
                    };
                    assert!(matches!(prop.as_str(), "channel" | "role"), "{prop}");
                    let node = file.graph.nodes.get_mut(&problem.node.unwrap()).unwrap();
                    node.props.insert(prop.clone(), "123456789012345678".into());
                }
                assert_eq!(registry.validate(&file.graph), Vec::new());
                assert_eq!(file.layout.nodes.len(), file.graph.nodes.len());
            }
            rustbot_codegen::generate(&project, &registry).unwrap();
            let reopened = rustbot_core::Project::open(&project.root).unwrap();
            assert_eq!(reopened.manifest.memory, project.manifest.memory);
        }
    }

    /// The bots the templates make build against djs-rs with clippy, their IDs filled in.
    /// Run by hand (see CLAUDE.md): CI's compile job does not build the app.
    #[test]
    #[ignore = "builds bots: needs network or a filled cargo cache"]
    fn template_bots_compile() {
        let target =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated-bots");
        for template in &TEMPLATES {
            let dir = tempfile::tempdir().unwrap();
            let mut project = super::create(&dir.path().join("bot"), template).unwrap();
            let mut registry = rustbot_core::Registry::with_core();
            registry.set_memory(project.manifest.memory.clone());
            for file in &mut project.graphs {
                for problem in registry.validate(&file.graph) {
                    if let ProblemKind::MissingProp(prop) = problem.kind {
                        let node = file.graph.nodes.get_mut(&problem.node.unwrap()).unwrap();
                        node.props.insert(prop, "123456789012345678".into());
                    }
                }
            }
            let output = rustbot_codegen::generate(&project, &registry).unwrap();
            rustbot_codegen::write(&project, &output).unwrap();
            let status = std::process::Command::new(env!("CARGO"))
                .args(["clippy", "--quiet", "--", "-D", "warnings"])
                .current_dir(project.root.join("generated"))
                .env("CARGO_TARGET_DIR", &target)
                .status()
                .unwrap();
            assert!(status.success(), "{}", template.id);
        }
    }
}
