use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DJS_RS_VERSION, SCHEMA};

/// `project.toml`, the project's settings (D-051).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Manifest {
    /// The project format version.
    pub schema: u32,
    /// Identifies the project, e.g. for its secrets in the keyring (D-050).
    pub id: Uuid,
    /// The bot's own settings.
    pub bot: BotSettings,
    /// The djs-rs dependency of the generated code.
    pub djs_rs: DjsRs,
    /// Gateway settings.
    #[serde(default)]
    pub gateway: Gateway,
}

/// The `[bot]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BotSettings {
    /// The name shown in the app.
    pub name: String,
    /// The Rust package name of the generated bot.
    pub package: String,
    /// A short description.
    #[serde(default)]
    pub description: String,
    /// The bot's license as an SPDX expression. Empty means none (D-023).
    #[serde(default)]
    pub license: String,
}

/// The `[djs-rs]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DjsRs {
    /// The djs-rs version the generated code depends on.
    pub version: String,
    /// Extra cargo features of djs-rs, e.g. `voice`.
    #[serde(default)]
    pub features: Vec<String>,
}

/// The `[gateway]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Gateway {
    /// Intents added by hand. The rest are computed from the graph (D-049).
    #[serde(default)]
    pub extra_intents: Vec<String>,
}

impl Manifest {
    /// Settings for a new project with a fresh random id.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            schema: SCHEMA,
            id: Uuid::new_v4(),
            bot: BotSettings {
                name: name.to_owned(),
                package: package_name(name),
                description: String::new(),
                license: String::new(),
            },
            djs_rs: DjsRs {
                version: DJS_RS_VERSION.to_owned(),
                features: Vec::new(),
            },
            gateway: Gateway::default(),
        }
    }
}

/// A Rust package name derived from a display name: `"Ping Bot!"` becomes `"ping-bot"`.
// shortcut: non-ASCII letters become '-' (no transliteration); the user can edit the name.
fn package_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    match out.chars().next() {
        None => "bot".to_owned(),
        Some(c) if c.is_ascii_digit() => format!("bot-{out}"),
        Some(_) => out.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::package_name;

    #[test]
    fn package_names() {
        assert_eq!(package_name("Ping Bot!"), "ping-bot");
        assert_eq!(package_name("  my__cool   bot "), "my-cool-bot");
        assert_eq!(package_name("Mój Bot"), "m-j-bot");
        assert_eq!(package_name("123"), "bot-123");
        assert_eq!(package_name("!!!"), "bot");
    }
}
