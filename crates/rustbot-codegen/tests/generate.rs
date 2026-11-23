//! Code generation from a project (D-012, D-057–D-059).

use std::fs;
use std::path::Path;
use std::time::Instant;

use proc_macro2::{TokenStream, TokenTree};
use rustbot_codegen::{Error, generate, write};
use rustbot_core::{Graph, GraphFile, GraphKind, Layout, Manifest, Project, Registry};

/// A `/<name>` command replying `content`.
fn command(name: &str, description: &str, content: &str) -> GraphFile {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command",
                    "props": { "name": name, "description": description } },
            "n2": { "type": "core.reply", "props": { "content": content } }
        },
        "edges": [ { "from": "n1.then", "to": "n2.run" } ]
    }))
    .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: name.to_owned(),
        graph,
        layout: Layout::default(),
    }
}

/// `/ban` with an option of every type (D-076) and a reply that reads all of them.
fn ban() -> GraphFile {
    let mut file = command(
        "ban",
        "Bans someone",
        "{option.cel.name} {option.cel.id} {option.cel.username} {option.cel.display_name} \
         {option.cel.mention} | {option.powód} {option.dni} {option.cicho} | \
         {option.kanał.id} {option.kanał.name} {option.kanał.mention} | \
         {option.rola.id} {option.rola.name} {option.rola.mention} {user.mention} @here",
    );
    let options = serde_json::json!([
        { "name": "cel", "type": "user", "description": "Who", "required": true },
        { "name": "powód", "type": "text", "description": "Why" },
        { "name": "dni", "type": "number", "description": "Days of messages to delete" },
        { "name": "cicho", "type": "bool", "description": "Do not announce it" },
        { "name": "kanał", "type": "channel", "description": "Where to log it" },
        { "name": "rola", "type": "role", "description": "A role to mention" }
    ]);
    let trigger = file
        .graph
        .nodes
        .get_mut(&rustbot_core::NodeId("n1".to_owned()))
        .unwrap();
    trigger.props.insert("options".to_owned(), options);
    file
}

/// `/check`: the D-014 example's conditions, nested (D-077, D-080, D-081).
fn conditions() -> GraphFile {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command", "props": {
                "name": "check", "description": "Checks", "options": [
                    { "name": "cel", "type": "user", "description": "Who", "required": true },
                    { "name": "powód", "type": "text", "description": "Why" },
                    { "name": "ile", "type": "number", "description": "How many" },
                    { "name": "rola", "type": "role", "description": "Role" } ] } },
            "n2": { "type": "core.has_permission",
                    "props": { "user": "user", "permission": "ban_members" } },
            "n3": { "type": "core.compare",
                    "props": { "left": "{option.powód}", "operator": "contains", "right": "spam" } },
            "n4": { "type": "core.reply", "props": { "content": "Spam!" } },
            "n5": { "type": "core.has_role",
                    "props": { "user": "option.cel", "role": "123456789012345678" } },
            "n6": { "type": "core.reply", "props": { "content": "{option.cel.name} is a moderator" } },
            "n7": { "type": "core.reply", "props": { "content": "No permission" } },
            "n8": { "type": "core.has_role",
                    "props": { "user": "user", "role": "option.rola" } },
            "n9": { "type": "core.compare",
                    "props": { "left": "{option.ile}", "operator": "greater", "right": "3" } },
            "n10": { "type": "core.reply", "props": { "content": "More than 3" } },
            "n11": { "type": "core.compare",
                    "props": { "left": "{option.powód}", "operator": "not_equals", "right": "" } }
        },
        "edges": [
            { "from": "n1.then", "to": "n2.run" },
            { "from": "n2.yes", "to": "n3.run" },
            { "from": "n3.yes", "to": "n4.run" },
            { "from": "n3.no", "to": "n5.run" },
            { "from": "n5.yes", "to": "n6.run" },
            { "from": "n2.no", "to": "n7.run" },
            { "from": "n1.then", "to": "n8.run" },
            { "from": "n8.no", "to": "n9.run" },
            { "from": "n9.yes", "to": "n10.run" },
            { "from": "n1.then", "to": "n11.run" }
        ]
    }))
    .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: "check".to_owned(),
        graph,
        layout: Layout::default(),
    }
}

/// `/moderate`: every moderation action, with "error" wired and not (D-073, D-082-D-084).
fn moderation() -> GraphFile {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command", "props": {
                "name": "moderate", "description": "Moderates", "options": [
                    { "name": "cel", "type": "user", "description": "Who", "required": true },
                    { "name": "powód", "type": "text", "description": "Why" },
                    { "name": "minuty", "type": "number", "description": "How long" },
                    { "name": "rola", "type": "role", "description": "Role" } ] } },
            "n2": { "type": "core.ban", "props": {
                "user": "option.cel", "delete_messages": "day", "reason": "{option.powód}" } },
            "n3": { "type": "core.reply", "props": { "content": "Banned {option.cel.mention}" } },
            "n4": { "type": "core.reply", "props": { "content": "I can't ban them" } },
            "n5": { "type": "core.kick", "props": { "user": "option.cel", "reason": "" } },
            "n6": { "type": "core.reply", "props": { "content": "No kick" } },
            "n7": { "type": "core.timeout", "props": {
                "user": "option.cel", "minutes": "{option.minuty}", "reason": "Spam" } },
            "n8": { "type": "core.add_role", "props": {
                "user": "user", "role": "123456789012345678", "reason": "" } },
            "n9": { "type": "core.remove_role", "props": {
                "user": "option.cel", "role": "option.rola", "reason": "By {user.name}" } },
            "n10": { "type": "core.ban", "props": {
                "user": "user", "delete_messages": "none", "reason": "" } }
        },
        "edges": [
            { "from": "n1.then", "to": "n2.run" },
            { "from": "n2.then", "to": "n3.run" },
            { "from": "n2.error", "to": "n4.run" },
            { "from": "n1.then", "to": "n5.run" },
            { "from": "n5.error", "to": "n6.run" },
            { "from": "n1.then", "to": "n7.run" },
            { "from": "n7.then", "to": "n8.run" },
            { "from": "n8.then", "to": "n9.run" },
            { "from": "n1.then", "to": "n10.run" }
        ]
    }))
    .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: "moderate".to_owned(),
        graph,
        layout: Layout::default(),
    }
}

/// `/announce`: messages to channels, embeds, a private reply and a follow-up (D-086-D-088).
fn messages() -> GraphFile {
    let graph: Graph = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "nodes": {
            "n1": { "type": "core.slash_command", "props": {
                "name": "announce", "description": "Announces", "options": [
                    { "name": "kanał", "type": "channel", "description": "Where", "required": true },
                    { "name": "tekst", "type": "text", "description": "What", "required": true },
                    { "name": "kolor", "type": "text", "description": "Colour" },
                    { "name": "obraz", "type": "text", "description": "Image" } ] } },
            "n2": { "type": "core.send_message", "props": {
                "channel": "option.kanał", "content": "{option.tekst} @here",
                "embed_title": "Announcement", "embed_description": "{option.tekst}",
                "embed_color": "#5865F2", "embed_image": "{option.obraz}",
                "embed_footer": "By {user.name}" } },
            "n3": { "type": "core.reply", "props": {
                "content": "Sent to {option.kanał.mention}", "private": true } },
            "n4": { "type": "core.send_message", "props": {
                "channel": "channel", "content": "",
                "embed_description": "Log: {user.mention}", "embed_color": "{option.kolor}" } },
            "n5": { "type": "core.send_message", "props": {
                "channel": "123456789012345678", "content": "Fixed" } },
            "n6": { "type": "core.reply", "props": { "content": "Second" } }
        },
        "edges": [
            { "from": "n1.then", "to": "n2.run" },
            { "from": "n2.then", "to": "n3.run" },
            { "from": "n1.then", "to": "n4.run" },
            { "from": "n4.then", "to": "n5.run" },
            { "from": "n1.then", "to": "n6.run" }
        ]
    }))
    .unwrap();
    GraphFile {
        kind: GraphKind::Command,
        name: "announce".to_owned(),
        graph,
        layout: Layout::default(),
    }
}

/// A command graph from JSON.
fn command_graph(name: &str, graph: serde_json::Value) -> GraphFile {
    GraphFile {
        kind: GraphKind::Command,
        name: name.to_owned(),
        graph: serde_json::from_value(graph).unwrap(),
        layout: Layout::default(),
    }
}

/// Buttons and menus (D-097–D-100): a confirmation that waits, a role menu whose clicks
/// always work with a nested button, and both kinds in events.
#[allow(clippy::too_many_lines)] // the graphs, as data
fn clicks() -> Vec<GraphFile> {
    let id = "123456789012345678";
    let yes_no = serde_json::json!([
        { "id": "b1", "label": "Yes", "style": "danger" },
        { "id": "b2", "label": "No", "style": "secondary" },
        { "id": "b3", "label": "Rules", "style": "link", "url": "https://example.com/rules" }
    ]);
    vec![
        command_graph(
            "confirm",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.slash_command", "props": {
                        "name": "confirm", "description": "Bans after asking", "options": [
                            { "name": "target", "type": "user", "description": "Who", "required": true } ] } },
                    "n2": { "type": "core.reply", "props": {
                        "content": "Ban {option.target.mention}?", "private": true, "buttons": yes_no,
                        "wait": "5", "only_author": true, "not_yours": "Not your button." } },
                    "n3": { "type": "core.has_permission", "props": {
                        "user": "user", "permission": "ban_members" } },
                    "n4": { "type": "core.ban", "props": {
                        "user": "option.target", "delete_messages": "none" } },
                    "n5": { "type": "core.reply", "props": { "content": "Banned {option.target.name}" } },
                    "n6": { "type": "core.reply", "props": { "content": "Cancelled", "private": true } },
                    "n7": { "type": "core.reply", "props": { "content": "Too slow" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.b1", "to": "n3.run" },
                    { "from": "n3.yes", "to": "n4.run" },
                    { "from": "n4.then", "to": "n5.run" },
                    { "from": "n2.b2", "to": "n6.run" },
                    { "from": "n2.timeout", "to": "n7.run" }
                ]
            }),
        ),
        command_graph(
            "roles",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.slash_command", "props": {
                        "name": "roles", "description": "Posts the role menu" } },
                    "n2": { "type": "core.send_message", "props": {
                        "channel": "channel", "content": "Pick a colour",
                        "menu": [ { "id": "o1", "label": "Red" }, { "id": "o2", "label": "Blue" } ],
                        "menu_placeholder": "Colours",
                        "buttons": [ { "id": "b1", "label": "Mine only", "style": "primary" } ],
                        "only_author": true } },
                    "n3": { "type": "core.reply", "props": { "content": "Posted", "private": true } },
                    "n4": { "type": "core.add_role", "props": { "user": "user", "role": id } },
                    "n5": { "type": "core.reply", "props": {
                        "content": "{user.name} is red now in {channel.name} on {server.name}",
                        "private": true } },
                    "n6": { "type": "core.remove_role", "props": { "user": "user", "role": id } },
                    "n7": { "type": "core.reply", "props": { "content": "Again?",
                        "buttons": [ { "id": "b1", "label": "Again", "style": "success" } ] } },
                    "n8": { "type": "core.reply", "props": { "content": "Again, {user.mention}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.then", "to": "n3.run" },
                    { "from": "n2.o1", "to": "n4.run" },
                    { "from": "n4.then", "to": "n5.run" },
                    { "from": "n2.o2", "to": "n6.run" },
                    { "from": "n2.b1", "to": "n7.run" },
                    { "from": "n7.b1", "to": "n8.run" }
                ]
            }),
        ),
        event_graph(
            "verify",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.member_join" },
                    "n2": { "type": "core.send_message", "props": { "channel": id,
                        "content": "Welcome {user.mention}, press to verify",
                        "buttons": [ { "id": "b1", "label": "Verify", "style": "success" } ] } },
                    "n3": { "type": "core.add_role", "props": { "user": "user", "role": id } },
                    "n4": { "type": "core.reply", "props": { "content": "Verified", "private": true } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.b1", "to": "n3.run" },
                    { "from": "n3.then", "to": "n4.run" }
                ]
            }),
        ),
        event_graph(
            "ask_delete",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.message" },
                    "n2": { "type": "core.reply", "props": { "content": "Delete {message}?",
                        "wait": "{message}", "only_author": true,
                        "buttons": [ { "id": "b1", "label": "Delete", "style": "danger" } ] } },
                    "n3": { "type": "core.delete_message" },
                    "n4": { "type": "core.reply", "props": { "content": "Deleted", "private": true } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.b1", "to": "n3.run" },
                    { "from": "n3.then", "to": "n4.run" }
                ]
            }),
        ),
    ]
}

/// The actions of D-102 on the messages of D-103: a command that reacts to, pins, edits
/// and unpins its own reply, sends DMs and renames, and an event working on its message.
#[allow(clippy::too_many_lines)] // the graphs, as data
fn actions() -> Vec<GraphFile> {
    vec![
        command_graph(
            "tools",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.slash_command", "props": {
                        "name": "tools", "description": "Uses every action", "options": [
                            { "name": "target", "type": "user", "description": "Who", "required": true },
                            { "name": "emoji", "type": "text", "description": "Which" },
                            { "name": "nick", "type": "text", "description": "Name" } ] } },
                    "n2": { "type": "core.reply", "props": { "content": "Working…" } },
                    "n3": { "type": "core.react", "props": { "message": "last", "emoji": "👍" } },
                    "n4": { "type": "core.react", "props": { "message": "last", "emoji": "{option.emoji}" } },
                    "n5": { "type": "core.pin", "props": { "message": "last" } },
                    "n6": { "type": "core.wait", "props": { "seconds": "2" } },
                    "n7": { "type": "core.edit_message", "props": { "message": "last",
                        "content": "Done, {user.name}", "embed_title": "Result" } },
                    "n8": { "type": "core.send_dm", "props": { "user": "option.target",
                        "content": "Hello from {server.name}",
                        "buttons": [ { "id": "b1", "label": "Thanks", "style": "success" } ] } },
                    "n9": { "type": "core.reply", "props": { "content": "Their DMs are closed",
                        "private": true } },
                    "n10": { "type": "core.set_nickname", "props": { "user": "option.target",
                        "nickname": "{option.nick}", "reason": "Asked by {user.name}" } },
                    "n11": { "type": "core.unpin", "props": { "message": "last" } },
                    "n12": { "type": "core.reply", "props": { "content": "You're welcome" } },
                    "n13": { "type": "core.wait", "props": { "seconds": "{option.nick}" } },
                    "n14": { "type": "core.send_dm", "props": { "user": "user", "content": "Bye",
                        "wait": "1", "menu": [ { "id": "o1", "label": "Again" } ] } },
                    "n15": { "type": "core.set_nickname", "props": { "user": "user", "nickname": "" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.then", "to": "n3.run" },
                    { "from": "n3.then", "to": "n4.run" },
                    { "from": "n4.then", "to": "n5.run" },
                    { "from": "n5.then", "to": "n6.run" },
                    { "from": "n6.then", "to": "n7.run" },
                    { "from": "n7.then", "to": "n8.run" },
                    { "from": "n8.error", "to": "n9.run" },
                    { "from": "n8.then", "to": "n10.run" },
                    { "from": "n10.then", "to": "n11.run" },
                    { "from": "n8.b1", "to": "n12.run" },
                    { "from": "n11.then", "to": "n13.run" },
                    { "from": "n13.then", "to": "n14.run" },
                    { "from": "n14.o1", "to": "n15.run" }
                ]
            }),
        ),
        event_graph(
            "starboard",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.reaction_add" },
                    "n2": { "type": "core.compare", "props": {
                        "left": "{emoji}", "operator": "equals", "right": "⭐" } },
                    "n3": { "type": "core.pin", "props": { "message": "event" } },
                    "n4": { "type": "core.react", "props": { "message": "event",
                        "emoji": "<:star:123456789012345678>" } },
                    "n5": { "type": "core.send_dm", "props": { "user": "user",
                        "content": "Pinned, {user.name}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.yes", "to": "n3.run" },
                    { "from": "n3.then", "to": "n4.run" },
                    { "from": "n3.error", "to": "n5.run" }
                ]
            }),
        ),
    ]
}

/// The memory of D-105-D-108: one variable of each scope.
fn memory() -> Vec<rustbot_core::MemoryVar> {
    use rustbot_core::{MemoryScope, MemoryType, MemoryVar};
    let var = |name: &str, ty, scope, initial: &str| MemoryVar {
        name: name.to_owned(),
        ty,
        scope,
        initial: initial.to_owned(),
    };
    vec![
        var("warnings", MemoryType::Number, MemoryScope::Member, ""),
        var("balance", MemoryType::Number, MemoryScope::User, "100"),
        var("motd", MemoryType::Text, MemoryScope::Global, "Welcome!"),
        var("log", MemoryType::Text, MemoryScope::Server, ""),
    ]
}

/// Graphs that read and change memory: warnings with a "Forgive" button, a shop, and events.
#[allow(clippy::too_many_lines)] // the graphs, as data
fn remembering() -> Vec<GraphFile> {
    vec![
        command_graph(
            "warn",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.slash_command", "props": {
                        "name": "warn", "description": "Warns a user", "options": [
                            { "name": "target", "type": "user", "description": "Who", "required": true } ] } },
                    "n2": { "type": "core.memory_change", "props": {
                        "variable": "warnings", "user": "option.target", "amount": "1" } },
                    "n3": { "type": "core.reply", "props": { "private": true,
                        "content": "Could not save the warning" } },
                    "n4": { "type": "core.reply", "props": {
                        "content": "{option.target.mention} has {memory.warnings.option.target} warnings. {memory.log}",
                        "wait": "5",
                        "buttons": [ { "id": "b1", "label": "Forgive", "style": "secondary" } ] } },
                    "n5": { "type": "core.memory_reset", "props": {
                        "variable": "warnings", "user": "option.target" } },
                    "n6": { "type": "core.reply", "props": { "content": "Forgiven" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.error", "to": "n3.run" },
                    { "from": "n2.then", "to": "n4.run" },
                    { "from": "n4.b1", "to": "n5.run" },
                    { "from": "n5.then", "to": "n6.run" }
                ]
            }),
        ),
        command_graph(
            "shop",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.slash_command", "props": {
                        "name": "shop", "description": "Buys a greeting", "options": [
                            { "name": "text", "type": "text", "description": "The greeting" },
                            { "name": "price", "type": "number", "description": "How much" } ] } },
                    "n2": { "type": "core.compare", "props": {
                        "left": "{memory.balance}", "operator": "greater", "right": "{option.price}" } },
                    "n3": { "type": "core.memory_change", "props": {
                        "variable": "balance", "amount": "-{option.price}" } },
                    "n4": { "type": "core.memory_set", "props": {
                        "variable": "motd", "value": "{option.text}" } },
                    "n5": { "type": "core.memory_set", "props": {
                        "variable": "log", "value": "Bought by {user.name}" } },
                    "n6": { "type": "core.reply", "props": {
                        "content": "Done, {memory.balance} left. @everyone {memory.motd}" } },
                    "n7": { "type": "core.reply", "props": { "content": "You have {memory.balance}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.yes", "to": "n3.run" },
                    { "from": "n2.no", "to": "n7.run" },
                    { "from": "n3.then", "to": "n4.run" },
                    { "from": "n4.then", "to": "n5.run" },
                    { "from": "n5.then", "to": "n6.run" }
                ]
            }),
        ),
        event_graph(
            "startup",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.ready" },
                    "n2": { "type": "core.memory_set", "props": { "variable": "motd", "value": "Hello" } }
                },
                "edges": [ { "from": "n1.then", "to": "n2.run" } ]
            }),
        ),
        event_graph(
            "earn",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.message" },
                    "n2": { "type": "core.memory_change", "props": { "variable": "balance", "amount": "0.5" } },
                    "n3": { "type": "core.memory_set", "props": { "variable": "warnings", "value": "{memory.balance}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.then", "to": "n3.run" }
                ]
            }),
        ),
    ]
}

/// An event graph from JSON.
fn event_graph(name: &str, graph: serde_json::Value) -> GraphFile {
    GraphFile {
        kind: GraphKind::Event,
        name: name.to_owned(),
        graph: serde_json::from_value(graph).unwrap(),
        layout: Layout::default(),
    }
}

/// One graph per event (D-091-D-095): a welcome with a role, a goodbye, link moderation,
/// a start-up note and reaction roles.
#[allow(clippy::too_many_lines)] // one graph per event, as data
fn events() -> Vec<GraphFile> {
    let id = "123456789012345678";
    vec![
        event_graph(
            "member_join",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.member_join" },
                    "n2": { "type": "core.send_message", "props": { "channel": id,
                        "content": "Welcome {user.mention} to {server.name}, {user.name}!" } },
                    "n3": { "type": "core.add_role", "props": { "user": "user", "role": id } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.then", "to": "n3.run" }
                ]
            }),
        ),
        event_graph(
            "member_leave",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.member_leave" },
                    "n2": { "type": "core.send_message", "props": { "channel": id,
                        "content": "Bye {user.name} ({user.username}, {user.id})" } }
                },
                "edges": [ { "from": "n1.then", "to": "n2.run" } ]
            }),
        ),
        event_graph(
            "message",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.message" },
                    "n2": { "type": "core.compare", "props": {
                        "left": "{message}", "operator": "contains", "right": "http" } },
                    "n3": { "type": "core.delete_message" },
                    "n4": { "type": "core.timeout", "props": { "user": "user", "minutes": "10" } },
                    "n5": { "type": "core.send_message", "props": { "channel": "channel",
                        "content": "{user.mention}, no links in {channel.mention}." } },
                    "n6": { "type": "core.has_role", "props": { "user": "user", "role": id } },
                    "n7": { "type": "core.reply", "props": {
                        "content": "Hi {user.name} in {channel.name} on {server.id}",
                        "embed_title": "You said", "embed_description": "{message}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.yes", "to": "n3.run" },
                    { "from": "n3.then", "to": "n4.run" },
                    { "from": "n4.error", "to": "n5.run" },
                    { "from": "n2.no", "to": "n6.run" },
                    { "from": "n6.yes", "to": "n7.run" }
                ]
            }),
        ),
        event_graph(
            "echo",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.message", "props": { "include_bots": true } },
                    "n2": { "type": "core.reply", "props": { "content": "@here {message}" } }
                },
                "edges": [ { "from": "n1.then", "to": "n2.run" } ]
            }),
        ),
        event_graph(
            "ready",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.ready" },
                    "n2": { "type": "core.send_message", "props": { "channel": id, "content": "Online" } }
                },
                "edges": [ { "from": "n1.then", "to": "n2.run" } ]
            }),
        ),
        event_graph(
            "reaction_add",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.reaction_add" },
                    "n2": { "type": "core.compare", "props": {
                        "left": "{emoji}", "operator": "equals", "right": "👍" } },
                    "n3": { "type": "core.add_role", "props": { "user": "user", "role": id } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.yes", "to": "n3.run" }
                ]
            }),
        ),
        event_graph(
            "reaction_remove",
            serde_json::json!({
                "schema": 1,
                "nodes": {
                    "n1": { "type": "core.reaction_remove" },
                    "n2": { "type": "core.remove_role", "props": { "user": "user", "role": id } },
                    "n3": { "type": "core.send_message", "props": { "channel": "channel",
                        "content": "{user.name} took back {emoji}" } }
                },
                "edges": [
                    { "from": "n1.then", "to": "n2.run" },
                    { "from": "n2.then", "to": "n3.run" }
                ]
            }),
        ),
        event_graph(
            "quiet",
            serde_json::json!({
                "schema": 1,
                "nodes": { "n1": { "type": "core.ready" } },
                "edges": []
            }),
        ),
    ]
}

/// A project on disk with these graphs, opened again as the app would.
fn project(dir: &Path, manifest: Manifest, graphs: &[GraphFile]) -> Project {
    let project = Project::create(dir, manifest).unwrap();
    for graph in graphs {
        project.save_graph(graph).unwrap();
    }
    Project::open(dir).unwrap()
}

fn ping_project(dir: &Path) -> Project {
    let mut manifest = Manifest::new("Ping Bot");
    manifest.id = "3f2b6c1e-8a4d-4f7e-9b1a-2c5d7e8f9a0b".parse().unwrap();
    project(
        dir,
        manifest,
        &[command("ping", "Replies with Pong!", "Pong, {user.name}!")],
    )
}

#[test]
fn ping_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let output = generate(&ping_project(dir.path()), &Registry::with_core()).unwrap();
    let names: Vec<_> = output
        .files
        .keys()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(
        names,
        [
            "generated/Cargo.toml",
            "generated/rust-toolchain.toml",
            "generated/src/commands/mod.rs",
            "generated/src/commands/ping.rs",
            "generated/src/main.rs",
            "generated/src/settings.rs",
        ]
    );
    for (path, text) in &output.files {
        insta::assert_snapshot!(path.display().to_string(), text);
    }
}

#[test]
fn options_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Ban Bot"), &[ban()]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    insta::assert_snapshot!(
        "options",
        &output.files[Path::new("generated/src/commands/ban.rs")]
    );
}

/// A text option named `mention` is still text from users, and is made harmless (C-016).
#[test]
fn a_text_option_named_mention_is_quieted() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = command("say", "Says it", "@here {option.mention}");
    file.graph
        .nodes
        .get_mut(&rustbot_core::NodeId("n1".to_owned()))
        .unwrap()
        .props
        .insert(
            "options".to_owned(),
            serde_json::json!([{ "name": "mention", "type": "text", "description": "What" }]),
        );
    let p = project(dir.path(), Manifest::new("Say Bot"), &[file]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    let code =
        output.files[Path::new("generated/src/commands/say.rs")].replace(char::is_whitespace, "");
    assert!(code.contains("quiet(&option_0)"), "{code}");
}

/// Only the mentions the template's author wrote notify anyone (C-016).
#[test]
fn replies_notify_only_what_the_author_wrote() {
    let reply = |content: &str| {
        let dir = tempfile::tempdir().unwrap();
        let mut file = ban();
        file.graph
            .nodes
            .get_mut(&rustbot_core::NodeId("n2".to_owned()))
            .unwrap()
            .props
            .insert("content".to_owned(), content.into());
        let p = project(dir.path(), Manifest::new("Ban Bot"), &[file]);
        let output = generate(&p, &Registry::with_core()).unwrap();
        output.files[Path::new("generated/src/commands/ban.rs")].replace(char::is_whitespace, "")
    };
    let none = reply("{option.powód} {user.name}");
    assert!(none.contains(".allowed_mentions(djs_rs::model::message::AllowedMentions::default(),"));
    assert!(!none.contains("letmutmentions"));
    let some =
        reply("{user.mention} {option.cel.mention} {option.rola.mention} {option.kanał.mention}");
    assert!(some.contains("mentions.users.push(author.id);"));
    assert!(some.contains(r#"mentions.users.extend(cmd.get_user("cel").map(|user|user.id));"#));
    assert!(some.contains(r#"mentions.roles.extend(cmd.get_role("rola").map(|role|role.id));"#));
    assert!(!some.contains("AllowedMentionType::Everyone"));
    // With @here allowed, a user's "@everyone" in a value must not ping: values go through
    // `quiet`, and only then.
    let here = reply("@here {option.powód}");
    assert!(here.contains("AllowedMentionType::Everyone"));
    assert!(here.contains("quiet(&option_1)"));
    assert!(here.contains("fnquiet("));
    assert!(!none.contains("quiet"));
    // Mentions stay intact, and a value cannot complete an "@" in the template.
    let glued = reply("@here @{option.powód} {user.mention}");
    assert!(glued.contains("quiet(&option_1)"));
    assert!(!glued.contains("quiet(&user_mention)"));
    assert!(reply("@here {option.cel.mention}").contains("option_0_mention)"));
    assert!(!reply("@here {option.cel.mention}").contains("quiet(&option_0_mention)"));
}

#[test]
fn conditions_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Check Bot"), &[conditions()]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    insta::assert_snapshot!(
        "conditions",
        &output.files[Path::new("generated/src/commands/check.rs")]
    );
}

#[test]
fn moderation_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Mod Bot"), &[moderation()]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    insta::assert_snapshot!(
        "moderation",
        &output.files[Path::new("generated/src/commands/moderate.rs")]
    );
}

#[test]
fn messages_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("News Bot"), &[messages()]);
    let registry = Registry::with_core();
    assert_eq!(registry.validate(&p.graphs[0].graph), Vec::new());
    let output = generate(&p, &registry).unwrap();
    insta::assert_snapshot!(
        "messages",
        &output.files[Path::new("generated/src/commands/announce.rs")]
    );
}

#[test]
fn clicks_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Click Bot"), &clicks());
    let registry = Registry::with_core();
    for file in &p.graphs {
        assert_eq!(registry.validate(&file.graph), Vec::new(), "{}", file.name);
    }
    let output = generate(&p, &registry).unwrap();
    for (name, path) in [
        ("click_confirm", "commands/confirm.rs"),
        ("click_roles", "commands/roles.rs"),
        ("click_verify", "events/verify.rs"),
        ("click_ask_delete", "events/ask_delete.rs"),
    ] {
        insta::assert_snapshot!(name, &output.files[&Path::new("generated/src").join(path)]);
    }
}

#[test]
fn actions_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Tool Bot"), &actions());
    let registry = Registry::with_core();
    for file in &p.graphs {
        assert_eq!(registry.validate(&file.graph), Vec::new(), "{}", file.name);
    }
    let output = generate(&p, &registry).unwrap();
    for (name, path) in [
        ("action_tools", "commands/tools.rs"),
        ("action_starboard", "events/starboard.rs"),
    ] {
        insta::assert_snapshot!(name, &output.files[&Path::new("generated/src").join(path)]);
    }
}

#[test]
fn memory_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Shop Bot");
    manifest.memory = memory();
    let p = project(dir.path(), manifest, &remembering());
    // The caller's registry knows no memory; generate checks against the project's.
    let output = generate(&p, &Registry::with_core()).unwrap();
    for (name, path) in [
        ("memory_warn", "src/commands/warn.rs"),
        ("memory_shop", "src/commands/shop.rs"),
        ("memory_earn", "src/events/earn.rs"),
        ("memory_module", "src/memory.rs"),
        ("memory_main", "src/main.rs"),
        ("memory_cargo", "Cargo.toml"),
    ] {
        insta::assert_snapshot!(name, &output.files[&Path::new("generated").join(path)]);
    }
}

#[test]
fn events_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Event Bot"), &events());
    let registry = Registry::with_core();
    for file in &p.graphs {
        assert_eq!(registry.validate(&file.graph), Vec::new(), "{}", file.name);
    }
    let output = generate(&p, &registry).unwrap();
    for name in [
        "mod",
        "member_join",
        "member_leave",
        "message",
        "echo",
        "ready",
        "reaction_add",
        "reaction_remove",
        "quiet",
    ] {
        insta::assert_snapshot!(
            format!("event_{name}"),
            &output.files[Path::new(&format!("generated/src/events/{name}.rs"))]
        );
    }
    let main = &output.files[Path::new("generated/src/main.rs")];
    assert!(main.contains("mod events;") && main.contains("events::register(&client);"));
    // The intents the events need (D-049), privileged ones included (D-095).
    for intent in [
        "GUILD_MEMBERS",
        "GUILD_MESSAGES",
        "MESSAGE_CONTENT",
        "GUILD_MESSAGE_REACTIONS",
    ] {
        assert!(main.contains(&format!("Intents::{intent}")), "{intent}");
    }
}

#[test]
fn output_is_deterministic() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let registry = Registry::with_core();
    assert_eq!(
        generate(&ping_project(a.path()), &registry).unwrap(),
        generate(&ping_project(b.path()), &registry).unwrap()
    );
}

/// Every identifier in `tokens`, outside string literals.
fn idents(tokens: TokenStream, out: &mut Vec<String>) {
    for tree in tokens {
        match tree {
            TokenTree::Ident(ident) => out.push(ident.to_string()),
            TokenTree::Group(group) => idents(group.stream(), out),
            _ => {}
        }
    }
}

/// Project content from an untrusted repository must stay data.
#[test]
fn content_cannot_inject_code() {
    let evil = r#"x"); std::process::exit(1); ("{{}} \" \\ {"#;
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.bot.package = "bot\"\n[dependencies]\nevil = \"1\"".to_owned();
    let project = project(
        dir.path(),
        manifest,
        &[command(
            "ping",
            evil,
            &format!(
                "{} {{user.name}}",
                evil.replace('{', "{{").replace('}', "}}")
            ),
        )],
    );
    let output = generate(&project, &Registry::with_core()).unwrap();

    let code = &output.files[Path::new("generated/src/commands/ping.rs")];
    let tokens: TokenStream = code.parse().unwrap();
    let mut found = Vec::new();
    idents(tokens, &mut found);
    assert!(!found.contains(&"exit".to_owned()), "{code}");

    let cargo: toml::Table = output.files[Path::new("generated/Cargo.toml")]
        .parse()
        .unwrap();
    assert_eq!(
        cargo["package"]["name"].as_str(),
        Some(project.manifest.bot.package.as_str())
    );
    assert_eq!(cargo["dependencies"].as_table().unwrap().len(), 3);
}

#[test]
fn literal_braces_stay_literal() {
    let dir = tempfile::tempdir().unwrap();
    let project = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "{{user.name}} is {user.name}")],
    );
    let output = generate(&project, &Registry::with_core()).unwrap();
    let code = &output.files[Path::new("generated/src/commands/ping.rs")];
    assert!(
        code.contains(r#"format!("{{user.name}} is {}", user_name)"#),
        "{code}"
    );
}

#[test]
fn errors() {
    let registry = Registry::with_core();

    let dir = tempfile::tempdir().unwrap();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "{usr}")],
    );
    assert!(matches!(
        generate(&p, &registry),
        Err(Error::Invalid { .. })
    ));

    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.gateway.extra_intents = vec!["EVERYTHING".to_owned()];
    let p = project(dir.path(), manifest, &[command("ping", "d", "hi")]);
    assert!(matches!(generate(&p, &registry), Err(Error::UnknownIntent(i)) if i == "EVERYTHING"));

    let dir = tempfile::tempdir().unwrap();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("a-b", "d", "hi"), command("a_b", "d", "hi")],
    );
    assert!(matches!(generate(&p, &registry), Err(Error::NameClash(..))));

    let dir = tempfile::tempdir().unwrap();
    let mut looped = command("ping", "d", "hi");
    looped.graph.edges.push(rustbot_core::Edge {
        from: "n2.then".parse().unwrap(),
        to: "n2.run".parse().unwrap(),
    });
    let p = project(dir.path(), Manifest::new("Bot"), &[looped]);
    assert!(matches!(generate(&p, &registry), Err(Error::Cycle { .. })));
}

#[test]
fn intents_are_written_from_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Bot");
    manifest.gateway.extra_intents = vec!["MESSAGE_CONTENT".to_owned(), "GUILDS".to_owned()];
    let p = project(dir.path(), manifest, &[command("ping", "d", "hi")]);
    let output = generate(&p, &Registry::with_core()).unwrap();
    let main = &output.files[Path::new("generated/src/main.rs")];
    assert!(
        main.contains("Intents::GUILDS | Intents::MESSAGE_CONTENT"),
        "{main}"
    );
}

#[test]
fn write_removes_stale_modules() {
    let dir = tempfile::tempdir().unwrap();
    let registry = Registry::with_core();
    let p = project(
        dir.path(),
        Manifest::new("Bot"),
        &[command("ping", "d", "hi"), command("old", "d", "hi")],
    );
    write(&p, &generate(&p, &registry).unwrap()).unwrap();
    assert!(dir.path().join("generated/src/commands/old.rs").exists());

    std::fs::remove_file(dir.path().join("graphs/commands/old.json")).unwrap();
    let p = Project::open(dir.path()).unwrap();
    write(&p, &generate(&p, &registry).unwrap()).unwrap();
    assert!(!dir.path().join("generated/src/commands/old.rs").exists());
    assert!(dir.path().join("generated/src/commands/ping.rs").exists());
}

/// The generated `/ping` bot compiles against the real djs-rs from crates.io. Needs the
/// network the first time: `cargo test -p rustbot-codegen -- --ignored compiles`.
#[test]
#[ignore = "downloads and builds djs-rs"]
fn generated_bot_compiles() {
    let dir = tempfile::tempdir().unwrap();
    let mut graphs = vec![command(
        "ping",
        "Replies with Pong!",
        "Pong, {user.name}! {user.id} {user.username} {user.display_name} {user.mention}",
    )];
    graphs.push(command(
        "where",
        "Says where it is",
        "{channel.id} {channel.name} {channel.mention} {server.id} {server.name}",
    ));
    graphs.push(ban());
    graphs.push(conditions());
    graphs.push(moderation());
    graphs.push(messages());
    graphs.extend(events());
    graphs.extend(clicks());
    graphs.extend(actions());
    graphs.extend(remembering());
    let mut manifest = Manifest::new("Ping Bot");
    manifest.memory = memory();
    let p = project(dir.path(), manifest, &graphs);
    write(&p, &generate(&p, &Registry::with_core()).unwrap()).unwrap();

    // One target folder for every run, so djs-rs is built only once.
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated-bots");
    let status = std::process::Command::new(env!("CARGO"))
        .args(["clippy", "--quiet", "--", "-D", "warnings"])
        .current_dir(dir.path().join("generated"))
        .env("CARGO_TARGET_DIR", target)
        .status()
        .unwrap();
    assert!(status.success());
}

/// D-038: generating 500 nodes takes at most 100 ms. Measured in release:
/// `cargo test -p rustbot-codegen --release -- --ignored budget --nocapture`.
#[test]
#[ignore = "a measurement, run in release"]
fn budget_500_nodes() {
    let mut file = command("big", "d", "{user.name}");
    for i in 3..=501 {
        let node = file.graph.nodes[&rustbot_core::NodeId("n2".to_owned())].clone();
        file.graph
            .nodes
            .insert(rustbot_core::NodeId(format!("n{i}")), node);
        file.graph.edges.push(rustbot_core::Edge {
            from: format!("n{}.then", i - 1).parse().unwrap(),
            to: format!("n{i}.run").parse().unwrap(),
        });
    }
    assert_eq!(file.graph.nodes.len(), 501);
    let dir = tempfile::tempdir().unwrap();
    let p = project(dir.path(), Manifest::new("Bot"), &[file]);
    let registry = Registry::with_core();

    let runs = 20;
    let started = Instant::now();
    for _ in 0..runs {
        generate(&p, &registry).unwrap();
    }
    let each = started.elapsed() / runs;
    println!("generate, 501 nodes: {each:?} per run");
    assert!(each.as_millis() <= 100, "{each:?}");

    // The editor marks problem nodes, so it checks the graph as it draws (D-090).
    let graph = &p.graphs[0].graph;
    let started = Instant::now();
    for _ in 0..runs {
        assert_eq!(
            registry.validate(graph).len() + registry.warnings(graph).len(),
            0
        );
    }
    let each = started.elapsed() / runs;
    println!("validate and warnings, 501 nodes: {each:?} per run");
}

/// Files a cloned project hides in `generated/` would run at build time; they are removed.
#[test]
fn write_owns_all_of_generated() {
    let dir = tempfile::tempdir().unwrap();
    let p = ping_project(dir.path());
    for planted in [
        "generated/build.rs",
        "generated/.cargo/config.toml",
        "generated/examples/x.rs",
    ] {
        p.write_file(Path::new(planted), "planted").unwrap();
    }
    p.write_file(Path::new("generated/Cargo.lock"), "lock")
        .unwrap();
    p.write_file(Path::new("generated/target/debug/x"), "build output")
        .unwrap();

    write(&p, &generate(&p, &Registry::with_core()).unwrap()).unwrap();
    let left: Vec<String> = p
        .files_under(Path::new("generated"), &[])
        .unwrap()
        .iter()
        .map(|f| f.display().to_string())
        .collect();
    assert_eq!(
        left,
        [
            "generated/Cargo.lock",
            "generated/Cargo.toml",
            "generated/rust-toolchain.toml",
            "generated/src/commands/mod.rs",
            "generated/src/commands/ping.rs",
            "generated/src/main.rs",
            "generated/src/settings.rs",
            "generated/target/debug/x",
        ]
    );
}

/// The generated `memory.rs` keeps values across restarts and never overwrites a damaged
/// file (D-105). Named for the CI job that runs `--ignored compiles`.
#[test]
#[ignore = "builds and runs a crate: needs network or a filled cargo cache"]
fn generated_memory_compiles_and_keeps_values() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::new("Shop Bot");
    manifest.memory = memory();
    let p = project(dir.path(), manifest, &remembering());
    let output = generate(&p, &Registry::with_core()).unwrap();
    let krate = dir.path().join("check");
    fs::create_dir_all(krate.join("src")).unwrap();
    fs::write(
        krate.join("Cargo.toml"),
        "[package]\nname = \"check\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [workspace]\n\n[dependencies]\nserde_json = \"1\"\n",
    )
    .unwrap();
    fs::write(
        krate.join("src/memory.rs"),
        &output.files[Path::new("generated/src/memory.rs")],
    )
    .unwrap();
    fs::write(
        krate.join("src/main.rs"),
        r#"mod memory;
fn main() {
    let step = std::env::args().nth(1).unwrap();
    if let Err(error) = memory::load() {
        println!("{error}");
        std::process::exit(2);
    }
    match step.as_str() {
        "first" => {
            assert_eq!(memory::read("balance", "7", "100"), "100");
            memory::change("balance", "7", -2.5, 100.0).unwrap();
            memory::change("balance", "7", 0.5, 100.0).unwrap();
            memory::set("motd", "", serde_json::Value::String("Hi".into())).unwrap();
            memory::change("warnings", "1:7", 1.0, 0.0).unwrap();
            memory::reset("warnings", "1:7").unwrap();
            memory::change("warnings", "1:7", 1.0, 0.0).unwrap();
            assert!(memory::change("warnings", "1:7", f64::MAX, 0.0).is_ok());
            assert!(memory::change("warnings", "1:7", f64::MAX, 0.0).is_err());
            memory::set("warnings", "1:7", memory::number(1.0)).unwrap();
        }
        "second" => {
            assert_eq!(memory::read("balance", "7", "100"), "98");
            assert_eq!(memory::read("motd", "", "Welcome!"), "Hi");
            assert_eq!(memory::read("warnings", "1:7", "0"), "1");
        }
        other => panic!("{other}"),
    }
}
"#,
    )
    .unwrap();
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/generated-bots");
    let run = |step: &str| {
        std::process::Command::new(env!("CARGO"))
            .args(["run", "--quiet", "--", step])
            .current_dir(&krate)
            .env("CARGO_TARGET_DIR", &target)
            .output()
            .unwrap()
    };
    for step in ["first", "second"] {
        let out = run(step);
        assert!(out.status.success(), "{step}: {out:?}");
    }
    let data = fs::read_to_string(krate.join("data.json")).unwrap();
    assert!(data.contains("\"1:7\": 1"), "{data}");
    assert!(!krate.join("data.json.tmp").exists());

    // A damaged file stops the bot and stays as it was.
    fs::write(krate.join("data.json"), "{ \"balance\": ").unwrap();
    let out = run("second");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("data.json is damaged"));
    assert_eq!(
        fs::read_to_string(krate.join("data.json")).unwrap(),
        "{ \"balance\": "
    );
}
