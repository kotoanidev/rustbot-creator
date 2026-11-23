//! The app's text in Fluent (D-024, D-037), in the system's language: Polish for `pl*`,
//! English otherwise. A message missing in Polish falls back to English.

use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};

const ENGLISH: &str = include_str!("../locales/en/main.ftl");
const POLISH: &str = include_str!("../locales/pl/main.ftl");

struct Text {
    language: &'static str,
    chosen: FluentBundle<FluentResource>,
    english: FluentBundle<FluentResource>,
}

static TEXT: OnceLock<Text> = OnceLock::new();

fn bundle(language: &str, source: &str) -> FluentBundle<FluentResource> {
    let id = language.parse().expect("a valid language tag");
    let mut bundle = FluentBundle::new_concurrent(vec![id]);
    // gpui would draw Fluent's invisible bidi isolation marks as boxes.
    bundle.set_use_isolating(false);
    let resource = FluentResource::try_new(source.to_owned()).expect("valid Fluent");
    bundle
        .add_resource(resource)
        .expect("no message is defined twice");
    bundle
}

fn text() -> &'static Text {
    TEXT.get_or_init(|| {
        let polish = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
            .is_some_and(|value| value.starts_with("pl"));
        let (language, source) = if polish {
            ("pl", POLISH)
        } else {
            ("en", ENGLISH)
        };
        Text {
            language,
            chosen: bundle(language, source),
            english: bundle("en", ENGLISH),
        }
    })
}

/// The language of the app's text, `en` or `pl`.
pub(crate) fn language() -> &'static str {
    text().language
}

/// The message `id`.
pub(crate) fn t(id: &str) -> String {
    format(id, None).unwrap_or_else(|| id.to_owned())
}

/// The message `id`, or `fallback` if no language has it, e.g. a plugin's node name.
pub(crate) fn t_or(id: &str, fallback: &str) -> String {
    format(id, None).unwrap_or_else(|| fallback.to_owned())
}

/// The message `id` with its variables, e.g. `t_with("error-open", &[("error", "…")])`.
pub(crate) fn t_with(id: &str, args: &[(&str, &str)]) -> String {
    let mut fluent = FluentArgs::new();
    for &(name, value) in args {
        fluent.set(name, value.to_owned());
    }
    format(id, Some(&fluent)).unwrap_or_else(|| id.to_owned())
}

fn format(id: &str, args: Option<&FluentArgs<'_>>) -> Option<String> {
    let text = text();
    [&text.chosen, &text.english]
        .into_iter()
        .find_map(|bundle| {
            let pattern = bundle.get_message(id)?.value()?;
            let mut errors = Vec::new();
            Some(
                bundle
                    .format_pattern(pattern, args, &mut errors)
                    .into_owned(),
            )
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    fn ids(source: &str) -> BTreeSet<&str> {
        source
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(id, _)| id.trim()))
            .filter(|id| !id.starts_with('#'))
            .collect()
    }

    #[test]
    fn both_languages_have_the_same_messages() {
        super::bundle("en", super::ENGLISH);
        super::bundle("pl", super::POLISH);
        assert_eq!(ids(super::ENGLISH), ids(super::POLISH));
    }

    /// The built-in nodes, their pins and fields have names, as plugins will supply theirs.
    #[test]
    fn core_nodes_are_named() {
        let english = ids(super::ENGLISH);
        for node in rustbot_core::Registry::with_core().iter() {
            let node_id = node.id.replace('.', "-");
            assert!(
                english.contains(format!("node-{node_id}").as_str()),
                "{}",
                node.id
            );
            for pin in &node.pins {
                assert!(
                    english.contains(format!("pin-{}", pin.name).as_str()),
                    "{}",
                    pin.name
                );
            }
            for prop in &node.props {
                let id = format!("prop-{node_id}-{}", prop.name);
                assert!(english.contains(id.as_str()), "{id}");
            }
        }
        // Every value of a fixed list has a name, e.g. `choice-permission-ban_members`.
        for (list, choices) in [
            ("permission", rustbot_core::registry::Choices::Permission),
            ("comparison", rustbot_core::registry::Choices::Comparison),
            (
                "delete_messages",
                rustbot_core::registry::Choices::DeleteMessages,
            ),
            (
                "message_target",
                rustbot_core::registry::Choices::MessageTarget,
            ),
        ] {
            for value in choices.values() {
                let id = format!("choice-{list}-{value}");
                assert!(english.contains(id.as_str()), "{id}");
            }
        }
    }

    /// Every `t("…")` and `t_with("…"` in the source names an English message.
    #[test]
    fn every_message_used_exists() {
        let english = ids(super::ENGLISH);
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut files = vec![std::path::PathBuf::from(dir)];
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for call in ["t(\"", "t_with(\"", "t_or(\""] {
                for (at, _) in source.match_indices(call) {
                    // Not `split("` or `x.t("`: the call must start a name.
                    let before = source[..at].chars().next_back();
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                        continue;
                    }
                    let rest = &source[at + call.len()..];
                    let id = &rest[..rest.find('"').unwrap()];
                    if id != "…" {
                        assert!(english.contains(id), "{}: {id}", path.display());
                    }
                }
            }
        }
    }
}
