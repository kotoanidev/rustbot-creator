//! Asking Discord about the bot, for the Developer Portal guide (D-129, D-131–D-133): who
//! the token's bot is, which privileged intents the portal has switched on, and which
//! servers the bot is on. Also the invite link with the permissions the project's nodes
//! need. The calls block, so they run in the background.

use std::collections::BTreeSet;

use rustbot_core::{Project, Registry};

const API: &str = "https://discord.com/api/v10";

/// The Developer Portal's list of applications.
pub(crate) const PORTAL: &str = "https://discord.com/developers/applications";

/// The privileged intents, which the portal must switch on (D-095), by their names in
/// node types, with the application flags that say they are on: the full one and the
/// "limited" one of bots on fewer than 100 servers.
const PRIVILEGED: [(&str, u64); 3] = [
    ("GUILD_PRESENCES", 1 << 12 | 1 << 13),
    ("GUILD_MEMBERS", 1 << 14 | 1 << 15),
    ("MESSAGE_CONTENT", 1 << 18 | 1 << 19),
];

/// The token's bot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Bot {
    pub(crate) name: String,
    pub(crate) application: u64,
    /// The privileged intents switched on in the portal.
    pub(crate) intents: BTreeSet<&'static str>,
}

/// A server the bot is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Server {
    pub(crate) id: String,
    pub(crate) name: String,
}

/// Why asking Discord failed.
#[derive(Debug)]
pub(crate) enum Error {
    /// Discord does not know the token.
    Token,
    /// No answer, or one that makes no sense.
    Network(String),
}

/// Who the token's bot is.
pub(crate) fn bot(token: &str) -> Result<Bot, Error> {
    #[derive(serde::Deserialize)]
    struct Application {
        id: String,
        name: String,
        #[serde(default)]
        flags: u64,
        bot: Option<User>,
    }
    #[derive(serde::Deserialize)]
    struct User {
        username: String,
    }
    let application: Application = get(token, "/oauth2/applications/@me")?;
    Ok(Bot {
        name: application.bot.map_or(application.name, |bot| bot.username),
        application: application
            .id
            .parse()
            .map_err(|_| Error::Network("an application ID that is not a number".to_owned()))?,
        intents: intents_on(application.flags),
    })
}

/// The servers the bot is on, by name.
pub(crate) fn servers(token: &str) -> Result<Vec<Server>, Error> {
    #[derive(serde::Deserialize)]
    struct Guild {
        id: String,
        name: String,
    }
    let guilds: Vec<Guild> = get(token, "/users/@me/guilds")?;
    let mut servers: Vec<Server> = guilds
        .into_iter()
        .map(|guild| Server {
            id: guild.id,
            name: guild.name,
        })
        .collect();
    servers.sort_by_key(|server| server.name.to_lowercase());
    Ok(servers)
}

fn get<T: serde::de::DeserializeOwned>(token: &str, path: &str) -> Result<T, Error> {
    let response = ureq::get(format!("{API}{path}"))
        .header("Authorization", format!("Bot {token}"))
        // Discord asks API clients to name themselves like this.
        .header(
            "User-Agent",
            "DiscordBot (https://github.com/kotoanidev/rustbot-creator, 0.1)",
        )
        .call();
    let mut response = match response {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(401)) => return Err(Error::Token),
        Err(error) => return Err(Error::Network(error.to_string())),
    };
    let text = response
        .body_mut()
        .with_config()
        .limit(1 << 20)
        .read_to_string()
        .map_err(|error| Error::Network(error.to_string()))?;
    serde_json::from_str(&text).map_err(|error| Error::Network(error.to_string()))
}

/// The privileged intents an application's flags switch on.
fn intents_on(flags: u64) -> BTreeSet<&'static str> {
    PRIVILEGED
        .iter()
        .filter(|(_, bits)| flags & bits != 0)
        .map(|&(name, _)| name)
        .collect()
}

/// The privileged intents the project's nodes and settings need.
pub(crate) fn needed_intents(project: &Project, registry: &Registry) -> BTreeSet<&'static str> {
    let mut used: BTreeSet<&str> = project
        .manifest
        .gateway
        .extra_intents
        .iter()
        .map(String::as_str)
        .collect();
    for file in &project.graphs {
        for node in file.graph.nodes.values() {
            if let Some(node_type) = registry.get(&node.kind) {
                used.extend(node_type.intents.iter().map(String::as_str));
            }
        }
    }
    PRIVILEGED
        .iter()
        .map(|&(name, _)| name)
        .filter(|name| used.contains(name))
        .collect()
}

/// The permission bits the bot needs on a server: to see and write in channels, and what
/// the project's nodes do.
pub(crate) fn permissions(project: &Project, registry: &Registry) -> u64 {
    const VIEW_CHANNEL: u64 = 1 << 10;
    const SEND_MESSAGES: u64 = 1 << 11;
    const MANAGE_MESSAGES: u64 = 1 << 13;
    const EMBED_LINKS: u64 = 1 << 14;
    const READ_MESSAGE_HISTORY: u64 = 1 << 16;
    const ADD_REACTIONS: u64 = 1 << 6;
    const CREATE_PUBLIC_THREADS: u64 = 1 << 35;
    const SEND_MESSAGES_IN_THREADS: u64 = 1 << 38;
    let mut bits = VIEW_CHANNEL | SEND_MESSAGES | EMBED_LINKS | READ_MESSAGE_HISTORY;
    for file in &project.graphs {
        for node in file.graph.nodes.values() {
            let Some(node_type) = registry.get(&node.kind) else {
                continue;
            };
            // What a member needs to make a node act is what the bot needs to do it.
            bits |= node_type.permission.as_deref().map_or(0, permission_bit);
            bits |= match node.kind.as_str() {
                "core.delete_message" | "core.pin" | "core.unpin" => MANAGE_MESSAGES,
                "core.react" => ADD_REACTIONS,
                "core.create_thread" => CREATE_PUBLIC_THREADS | SEND_MESSAGES_IN_THREADS,
                "core.add_to_thread" => SEND_MESSAGES_IN_THREADS,
                _ => 0,
            };
        }
    }
    bits
}

/// A permission of [`rustbot_core::registry::Choices::Permission`] as Discord's bit.
fn permission_bit(name: &str) -> u64 {
    1 << match name {
        "kick_members" => 1,
        "ban_members" => 2,
        "administrator" => 3,
        "manage_channels" => 4,
        "manage_guild" => 5,
        "view_audit_log" => 7,
        "manage_messages" => 13,
        "mention_everyone" => 17,
        "mute_members" => 22,
        "move_members" => 24,
        "manage_nicknames" => 27,
        "manage_roles" => 28,
        "manage_threads" => 34,
        "moderate_members" => 40,
        _ => return 0,
    }
}

/// The link that adds the bot to a server, with its commands and `permissions`.
pub(crate) fn invite_link(application: u64, permissions: u64) -> String {
    format!(
        "https://discord.com/oauth2/authorize?client_id={application}\
         &scope=bot+applications.commands&permissions={permissions}"
    )
}

/// The portal's page of the bot, where its token and intents are.
pub(crate) fn bot_page(application: u64) -> String {
    format!("{PORTAL}/{application}/bot")
}

#[cfg(test)]
mod tests {
    use rustbot_core::Registry;

    use crate::templates::TEMPLATES;

    fn project(id: &str) -> (tempfile::TempDir, rustbot_core::Project) {
        let dir = tempfile::tempdir().unwrap();
        let template = TEMPLATES.iter().find(|t| t.id == id).unwrap();
        let project = crate::templates::create(&dir.path().join("bot"), template).unwrap();
        (dir, project)
    }

    /// The invite asks for what the template's nodes do, and nothing it does not need.
    #[test]
    fn the_invite_asks_for_what_the_nodes_need() {
        let registry = Registry::with_core();
        let (_dir, moderation) = project("moderation");
        let bits = super::permissions(&moderation, &registry);
        for bit in [2, 1, 40, 10, 11, 14, 16] {
            assert!(bits & 1 << bit != 0, "bit {bit}");
        }
        assert_eq!(bits & 1 << 3, 0, "never administrator");
        let (_dir, empty) = project("empty");
        assert_eq!(
            super::permissions(&empty, &registry),
            1 << 10 | 1 << 11 | 1 << 14 | 1 << 16
        );
        assert_eq!(
            super::invite_link(42, 3072),
            "https://discord.com/oauth2/authorize?client_id=42\
             &scope=bot+applications.commands&permissions=3072"
        );
    }

    #[test]
    fn privileged_intents_are_needed_and_seen_switched_on() {
        let registry = Registry::with_core();
        let (_dir, welcome) = project("welcome");
        assert_eq!(
            super::needed_intents(&welcome, &registry),
            ["GUILD_MEMBERS"].into()
        );
        let (_dir, levels) = project("levels");
        assert_eq!(
            super::needed_intents(&levels, &registry),
            ["MESSAGE_CONTENT"].into()
        );
        // The "limited" flag of a small bot counts as on.
        assert_eq!(
            super::intents_on(1 << 15 | 1 << 18),
            ["GUILD_MEMBERS", "MESSAGE_CONTENT"].into()
        );
        assert!(super::intents_on(1 << 23).is_empty());
    }
}
