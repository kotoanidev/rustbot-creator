//! Turns a RustBot Creator project into Rust code on djs-rs (D-012, D-057–D-059).
//!
//! [`generate`] builds every file of `generated/` from a validated project, as tokens
//! formatted by `prettyplease` (D-058), so project content can never escape a string
//! literal. [`write`] puts them on disk. The output is deterministic: the same project
//! gives the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use rustbot_core::registry::Problem;
use rustbot_core::{GraphKind, NodeId, Project, Registry};

mod cargo;
mod command;
mod event;
mod memory;

/// The gateway intents djs-rs knows, by their `Intents` constant names (D-049).
const INTENTS: &[&str] = &[
    "GUILDS",
    "GUILD_MEMBERS",
    "GUILD_MODERATION",
    "GUILD_EXPRESSIONS",
    "GUILD_INTEGRATIONS",
    "GUILD_WEBHOOKS",
    "GUILD_INVITES",
    "GUILD_VOICE_STATES",
    "GUILD_PRESENCES",
    "GUILD_MESSAGES",
    "GUILD_MESSAGE_REACTIONS",
    "GUILD_MESSAGE_TYPING",
    "DIRECT_MESSAGES",
    "DIRECT_MESSAGE_REACTIONS",
    "DIRECT_MESSAGE_TYPING",
    "MESSAGE_CONTENT",
    "GUILD_SCHEDULED_EVENTS",
    "AUTO_MODERATION_CONFIGURATION",
    "AUTO_MODERATION_EXECUTION",
    "GUILD_MESSAGE_POLLS",
    "DIRECT_MESSAGE_POLLS",
];

/// Why a project cannot be generated.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A graph has problems; the "Problems" panel shows them.
    #[error("{graph}: {} problem(s), the first: {}", .problems.len(), .problems[0])]
    Invalid {
        /// The graph's file name.
        graph: String,
        /// What is wrong.
        problems: Vec<Problem>,
    },
    /// A node type has no code generator, e.g. one from a plugin that is not installed.
    #[error("{graph}: node {node} of type {kind:?} cannot be generated")]
    NoCodegen {
        /// The graph's file name.
        graph: String,
        /// The node.
        node: NodeId,
        /// Its type.
        kind: String,
    },
    /// The flow of a graph loops back on itself.
    #[error("{graph}: the flow loops back to node {node}")]
    Cycle {
        /// The graph's file name.
        graph: String,
        /// Where it loops.
        node: NodeId,
    },
    /// An intent in `project.toml` that djs-rs does not know.
    #[error("unknown gateway intent {0:?}")]
    UnknownIntent(String),
    /// Two graphs would become the same Rust module.
    #[error("graphs {0:?} and {1:?} would become the same Rust module")]
    NameClash(String, String),
}

/// The generated files, by path relative to the project folder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Output {
    /// Each file's contents.
    pub files: BTreeMap<PathBuf, String>,
}

/// Generates `generated/` for `project`.
///
/// # Errors
/// A graph with problems or without a generator for one of its nodes, a loop in a flow, an
/// unknown intent, or two graphs that map to one module.
#[allow(clippy::too_many_lines)] // one pass over the project, file by file
pub fn generate(project: &Project, registry: &Registry) -> Result<Output, Error> {
    // Graphs are checked against this project's memory, whatever the caller's registry has.
    let mut registry = registry.clone();
    registry.set_memory(project.manifest.memory.clone());
    let registry = &registry;
    let has_memory = !project.manifest.memory.is_empty();
    let mut intents: BTreeSet<&str> = BTreeSet::new();
    for intent in &project.manifest.gateway.extra_intents {
        intents.insert(intent);
    }
    let mut commands = BTreeMap::new();
    let mut events = BTreeMap::new();
    for file in &project.graphs {
        let problems = registry.validate(&file.graph);
        if !problems.is_empty() {
            return Err(Error::Invalid {
                graph: file.name.clone(),
                problems,
            });
        }
        for node in file.graph.nodes.values() {
            if let Some(node_type) = registry.get(&node.kind) {
                intents.extend(node_type.intents.iter().map(String::as_str));
            }
        }
        match file.kind {
            GraphKind::Command => {
                let module = module_name(&file.name);
                if let Some(other) = commands.insert(module.clone(), file) {
                    return Err(Error::NameClash(other.name.clone(), file.name.clone()));
                }
            }
            GraphKind::Event => {
                let module = module_name(&file.name);
                if let Some(other) = events.insert(module.clone(), file) {
                    return Err(Error::NameClash(other.name.clone(), file.name.clone()));
                }
            }
        }
    }
    if let Some(unknown) = intents.iter().find(|i| !INTENTS.contains(i)) {
        return Err(Error::UnknownIntent((*unknown).to_owned()));
    }

    let mut output = Output::default();
    let src = Path::new("generated/src");
    output.files.insert(
        "generated/Cargo.toml".into(),
        cargo::manifest(&project.manifest),
    );
    // A hand build of an export uses the same Rust as the app (D-062).
    output.files.insert(
        "generated/rust-toolchain.toml".into(),
        format!(
            "# Generated by RustBot Creator: do not edit, it is overwritten.\n\n\
             [toolchain]\nchannel = \"{}\"\n",
            rustbot_core::RUST_VERSION
        ),
    );
    output.files.insert(
        src.join("main.rs"),
        rust(&main_rs(&intents, !events.is_empty(), has_memory)),
    );
    if has_memory {
        output
            .files
            .insert(src.join("memory.rs"), rust(&memory::module()));
    }
    output
        .files
        .insert(src.join("settings.rs"), rust(&settings_rs()));
    let mut modules = Vec::new();
    for (module, file) in &commands {
        let code = command::module(file, registry)?;
        output
            .files
            .insert(src.join(format!("commands/{module}.rs")), rust(&code));
        modules.push(format_ident!("{module}"));
    }
    output
        .files
        .insert(src.join("commands/mod.rs"), rust(&commands_mod(&modules)));
    for (module, file) in &events {
        // Validation leaves exactly one trigger; in a hand-edited file it may be a command.
        let trigger = file.graph.nodes.iter().find(|(_, node)| {
            registry
                .get(&node.kind)
                .is_some_and(|t| t.category == rustbot_core::registry::Category::Trigger)
        });
        let Some((trigger, node)) = trigger else {
            continue;
        };
        let Some(kind) = event::Event::of(&node.kind) else {
            return Err(Error::NoCodegen {
                graph: file.name.clone(),
                node: trigger.clone(),
                kind: node.kind.clone(),
            });
        };
        let code = event::module(file, trigger, kind, registry.memory())?;
        output
            .files
            .insert(src.join(format!("events/{module}.rs")), rust(&code));
    }
    if !events.is_empty() {
        let modules = events.keys().cloned().collect();
        output
            .files
            .insert(src.join("events/mod.rs"), rust(&event::mod_rs(&modules)));
    }
    Ok(output)
}

/// Writes `output` into the project. `generated/` belongs to the generator: every other file
/// in it is removed, except cargo's `Cargo.lock` and `target/`. A cloned project therefore
/// cannot hide a `build.rs` or a `.cargo/config.toml` there that the graph does not show.
///
/// # Errors
/// An I/O error, or a symbolic link in the way.
pub fn write(project: &Project, output: &Output) -> Result<(), rustbot_core::Error> {
    // shortcut: every file is rewritten, so cargo rebuilds the bot even when nothing changed;
    // compare before writing once runs feel slow.
    for (path, text) in &output.files {
        project.write_file(path, text)?;
    }
    let generated = Path::new("generated");
    let keep = generated.join("Cargo.lock");
    for path in project.files_under(generated, &[&generated.join("target")])? {
        if path != keep && !output.files.contains_key(&path) {
            project.remove_file(&path)?;
        }
    }
    Ok(())
}

/// A Rust module name for a graph file name: `ping` stays `ping`, `My Cmd!` becomes `my_cmd_`.
fn module_name(graph: &str) -> String {
    let mut name: String = graph
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        name.insert_str(0, "command_");
    }
    // Keywords (`type`, `match`, …) are not identifiers.
    if syn::parse_str::<syn::Ident>(&name).is_err() {
        name.push_str("_command");
    }
    name
}

/// Logs in and runs; a refusal of the intents is explained in plain words (D-095).
fn login() -> TokenStream {
    quote! {
    if let Err(error) = client.login().await {
        // Discord refuses privileged intents the portal has not switched on (D-095).
        let refused = matches!(
            &error,
            djs_rs::Error::Gateway(gateway) if matches!(
                gateway.kind(),
                djs_rs::gateway::ReceiveErrorKind::FatallyClosed {
                    code: djs_rs::model::gateway::CloseCode::DisallowedIntents,
                    ..
                }
            )
        );
        if refused {
            eprintln!(
                "Discord refused the bot's intents. Switch on the privileged intents \
                 its events need (Server Members, Message Content) in the Developer \
                 Portal: your application, Bot, Privileged Gateway Intents."
            );
            eprintln!("https://discord.com/developers/applications");
        }
        return Err(error.into());
    }
    }
}

fn main_rs(intents: &BTreeSet<&str>, has_events: bool, has_memory: bool) -> TokenStream {
    let login = login();
    let (memory_mod, memory_load) = if has_memory {
        (
            quote!(
                mod memory;
            ),
            quote!(memory::load()?;),
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };
    let (events_mod, events_register) = if has_events {
        (
            quote!(
                mod events;
            ),
            quote!(events::register(&client);),
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };
    let intents = if intents.is_empty() {
        quote!(Intents::empty())
    } else {
        let names = intents.iter().map(|i| format_ident!("{i}"));
        quote!(#(Intents::#names)|*)
    };
    quote! {
        #![doc = " The bot. Generated by RustBot Creator from the project's graphs: do not edit,"]
        #![doc = " it is overwritten on every build."]

        mod commands;
        #events_mod
        #memory_mod
        mod settings;

        use std::sync::Arc;

        use djs_rs::model::id::GuildId;
        use djs_rs::model::oauth2::Scope;
        use djs_rs::prelude::*;
        use djs_rs::rest::oauth2::AuthorizeUrl;

        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .init();

            let token = settings::get("DISCORD_TOKEN").ok_or("DISCORD_TOKEN is not set")?;
            // Commands go to this server while testing, and everywhere without it (D-055).
            let test_guild: Option<GuildId> = settings::get("DISCORD_TEST_GUILD_ID")
                .map(|id| id.parse())
                .transpose()
                .map_err(|_| "DISCORD_TEST_GUILD_ID is not a server ID")?;

            #memory_load
            let client = Client::builder(token).intents(#intents).build();
            let definitions = Arc::new(commands::definitions()?);
            client.on(Events::ClientReady, move |ctx, ready| {
                let definitions = Arc::clone(&definitions);
                async move {
                    match test_guild {
                        // Discord refuses commands for a server the bot is not on (D-072).
                        Some(guild) if !ready.guilds.iter().any(|g| g.id == guild) => {
                            let invite = AuthorizeUrl::bot(ready.application.id)
                                .scopes([Scope::ApplicationsCommands])
                                .guild_id(guild)
                                .build();
                            // The link on a line of its own, which the Logs panel opens.
                            eprintln!(
                                "The bot is not on the test server {guild}, so its commands \
                                 cannot be added there. Check the server ID in the bot \
                                 settings, or invite the bot with the link below and run it \
                                 again:"
                            );
                            eprintln!("{invite}");
                        }
                        Some(guild) => {
                            ctx.guild_commands(guild).set(&definitions).await?;
                        }
                        None => {
                            ctx.commands().set(&definitions).await?;
                        }
                    }
                    println!("Ready as {}", ready.user.tag());
                    Ok(())
                }
            });
            commands::register(&client);
            #events_register

            let shutdown = client.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    shutdown.shutdown();
                }
            });
            #login
            Ok(())
        }
    }
}

fn settings_rs() -> TokenStream {
    quote! {
        #![doc = " Settings from the environment, or from a `.env` file in the current folder or a"]
        #![doc = " folder above it (D-056). Generated by RustBot Creator: do not edit."]

        /// The setting `key`. An empty value counts as unset.
        pub(crate) fn get(key: &str) -> Option<String> {
            if let Ok(value) = std::env::var(key) {
                return Some(value).filter(|value| !value.is_empty());
            }
            let mut dir = std::env::current_dir().ok()?;
            loop {
                if let Ok(text) = std::fs::read_to_string(dir.join(".env")) {
                    return text.lines().find_map(|line| {
                        let (name, value) = line.trim().split_once('=')?;
                        let value = value.trim().trim_matches('"');
                        (name.trim() == key && !value.is_empty()).then(|| value.to_owned())
                    });
                }
                if !dir.pop() {
                    return None;
                }
            }
        }
    }
}

fn commands_mod(modules: &[proc_macro2::Ident]) -> TokenStream {
    // A bot of events only has no commands to route (D-116's "Welcome").
    let client = if modules.is_empty() {
        format_ident!("_client")
    } else {
        format_ident!("client")
    };
    let routes = modules
        .iter()
        .map(|module| quote!(#module::register(#client);));
    quote! {
        #![doc = " The slash commands, one module per graph. Generated by RustBot Creator: do not edit."]

        #(mod #modules;)*

        use djs_rs::Client;
        use djs_rs::builders::ValidationError;
        use djs_rs::model::rest::CreateApplicationCommand;

        /// What is registered with Discord.
        pub(crate) fn definitions() -> Result<Vec<CreateApplicationCommand>, ValidationError> {
            Ok(Vec::from([#(#modules::definition()?),*]))
        }

        /// Routes each command to its handler.
        pub(crate) fn register(#client: &Client) {
            #(#routes)*
        }
    }
}

/// Formats tokens this crate built. A parse failure is a bug in this crate.
fn rust(tokens: &TokenStream) -> String {
    let file = syn::parse2(tokens.clone()).expect("generated code is valid Rust");
    prettyplease::unparse(&file)
}

#[cfg(test)]
mod tests {
    use super::module_name;

    #[test]
    fn module_names() {
        assert_eq!(module_name("ping"), "ping");
        assert_eq!(module_name("My Cmd!"), "my_cmd_");
        assert_eq!(module_name("8ball"), "command_8ball");
        assert_eq!(module_name("type"), "type_command");
        assert_eq!(module_name("zażółć"), "za____");
    }
}
