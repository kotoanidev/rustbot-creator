//! One event graph → one module (D-091): its nodes run when the event comes.

use std::collections::BTreeSet;

use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use rustbot_core::{GraphFile, NodeId};

use crate::Error;
use crate::command::Body;

/// The events a graph can start from, by their trigger node types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    MemberJoin,
    MemberLeave,
    Message,
    Ready,
    ReactionAdd,
    ReactionRemove,
}

impl Event {
    /// The event a trigger node type starts, if it is an event.
    pub(crate) fn of(kind: &str) -> Option<Self> {
        Some(match kind {
            "core.member_join" => Self::MemberJoin,
            "core.member_leave" => Self::MemberLeave,
            "core.message" => Self::Message,
            "core.ready" => Self::Ready,
            "core.reaction_add" => Self::ReactionAdd,
            "core.reaction_remove" => Self::ReactionRemove,
            _ => return None,
        })
    }

    /// The djs-rs `Events` constant.
    fn djs(self) -> proc_macro2::Ident {
        format_ident!(
            "{}",
            match self {
                Self::MemberJoin => "GuildMemberAdd",
                Self::MemberLeave => "GuildMemberRemove",
                Self::Message => "MessageCreate",
                Self::Ready => "ClientReady",
                Self::ReactionAdd => "MessageReactionAdd",
                Self::ReactionRemove => "MessageReactionRemove",
            }
        )
    }

    /// The `user` variable's ID, a `UserId`. Only events that provide `user` are asked.
    pub(crate) fn user_id(self) -> TokenStream {
        match self {
            Self::MemberJoin => quote!(event.member.user.id),
            Self::MemberLeave => quote!(event.new.user.id),
            Self::Message => quote!(event.author.id),
            Self::ReactionAdd | Self::ReactionRemove => quote!(event.user_id),
            Self::Ready => unreachable!("ready provides no user"),
        }
    }

    /// The event's server, an `Option<GuildId>`.
    pub(crate) fn guild_id(self) -> TokenStream {
        match self {
            Self::MemberJoin => quote!(Some(event.guild_id)),
            Self::MemberLeave => quote!(Some(event.new.guild_id)),
            Self::Message | Self::ReactionAdd | Self::ReactionRemove => quote!(event.guild_id),
            Self::Ready => quote!(None::<djs_rs::model::id::GuildId>),
        }
    }

    /// The `channel` variable's ID, a `ChannelId`. Only events that provide it are asked.
    pub(crate) fn channel_id(self) -> TokenStream {
        match self {
            Self::Message | Self::ReactionAdd | Self::ReactionRemove => quote!(event.channel_id),
            _ => unreachable!("{self:?} provides no channel"),
        }
    }

    /// The `user` variable's roles, an `Option<&Vec<RoleId>>`; none when the event does not
    /// carry them, so "Has role?" says no.
    pub(crate) fn roles(self) -> TokenStream {
        match self {
            Self::MemberJoin => quote!(Some(&event.member.roles)),
            Self::Message | Self::ReactionAdd => {
                quote!(event.member.as_ref().map(|member| &member.roles))
            }
            Self::MemberLeave | Self::ReactionRemove | Self::Ready => {
                quote!(None::<&Vec<djs_rs::model::id::RoleId>>)
            }
        }
    }

    /// The `let` that computes a variable from the event.
    fn binding(self, path: &str) -> TokenStream {
        let name = format_ident!("{}", path.replace('.', "_"));
        // The user as a `User`; reactions carry only the ID, so it is fetched.
        let user = match self {
            Self::MemberJoin => quote!(event.member.user),
            Self::MemberLeave => quote!(event.new.user),
            Self::Message => quote!(event.author),
            _ => quote!(ctx.users().fetch(event.user_id).await?),
        };
        let nick = match self {
            Self::MemberJoin => quote!(event.member.nick.as_deref()),
            Self::Message | Self::ReactionAdd => {
                quote!(
                    event
                        .member
                        .as_ref()
                        .and_then(|member| member.nick.as_deref())
                )
            }
            _ => quote!(None::<&str>),
        };
        let value = match path {
            "user.id" => {
                let id = self.user_id();
                quote!(#id.to_string())
            }
            "user.mention" => {
                let id = self.user_id();
                quote!(djs_rs::formatters::user_mention(#id).to_string())
            }
            "user.username" => quote!(#user.username.to_string()),
            "user.display_name" => quote!(#user.display_name().to_string()),
            // What people see in the chat: nickname, then display name (D-054).
            "user.name" => quote! {
                match #nick {
                    Some(nick) => nick.to_string(),
                    None => #user.display_name().to_string(),
                }
            },
            "channel.id" => {
                let id = self.channel_id();
                quote!(#id.to_string())
            }
            "channel.mention" => {
                let id = self.channel_id();
                quote!(djs_rs::formatters::channel_mention(#id).to_string())
            }
            "channel.name" => {
                let id = self.channel_id();
                quote! {
                    ctx.channels().fetch(#id).await?.name().unwrap_or_default().to_string()
                }
            }
            "server.id" => {
                let id = self.guild_id();
                quote!(#id.map(|id| id.to_string()).unwrap_or_default())
            }
            "server.name" => {
                let id = self.guild_id();
                quote! {
                    match #id {
                        Some(id) => ctx.guilds().fetch(id).await?.name.to_string(),
                        None => String::new(),
                    }
                }
            }
            "message" => quote!(event.content.to_string()),
            "emoji" => quote!(event.emoji.name().unwrap_or_default().to_string()),
            other => unreachable!("validation only lets known variables through, not {other}"),
        };
        quote!(let #name = #value;)
    }
}

/// The module for a validated event graph.
pub(crate) fn module(
    file: &GraphFile,
    trigger: &NodeId,
    event: Event,
) -> Result<TokenStream, Error> {
    let node = &file.graph.nodes[trigger];
    let mut code = Body::for_event(file, event);
    let body = code.after(trigger, "then")?;
    let (vars, needs_quiet) = code.finish();
    // shortcut: every variable is computed first, so a name used in one branch is fetched
    // for each event; compute them where they are used if a busy server hits rate limits.
    let bindings = vars.iter().map(|path| event.binding(path));
    // The bot never answers itself, and other bots only if asked (D-093).
    let skip = (event == Event::Message).then(|| {
        if node.props.get("include_bots") == Some(&serde_json::Value::Bool(true)) {
            quote! {
                if ctx.application_id().is_some_and(|app| app.get() == event.author.id.get()) {
                    return Ok(());
                }
            }
        } else {
            quote! {
                if event.author.bot {
                    return Ok(());
                }
            }
        }
    });
    let body = quote!(#skip #(#bindings)* #body);
    // Parameters the code does not use would be warnings.
    let ctx = if uses(&body, "ctx") {
        quote!(ctx)
    } else {
        quote!(_ctx)
    };
    let payload = if uses(&body, "event") {
        quote!(event)
    } else {
        quote!(_event)
    };
    let quiet = needs_quiet.then(crate::command::quiet_fn);
    let djs = event.djs();
    let doc = format!(
        " The `{}` event graph, from `graphs/events/{}.json`.",
        node.kind, file.name
    );
    Ok(quote! {
        #![doc = #doc]
        #![doc = " Generated by RustBot Creator: do not edit, it is overwritten."]
        // Nested conditions mirror the graph's branches.
        #![allow(clippy::collapsible_if, clippy::collapsible_else_if)]

        use djs_rs::prelude::*;

        #quiet

        pub(crate) fn register(client: &Client) {
            client.on(Events::#djs, |#ctx, #payload| async move {
                #body
                Ok(())
            });
        }
    })
}

/// Whether `code` mentions the identifier `name` anywhere.
fn uses(code: &TokenStream, name: &str) -> bool {
    code.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => ident == name,
        TokenTree::Group(group) => uses(&group.stream(), name),
        _ => false,
    })
}

/// `events/mod.rs`: registers every event graph.
pub(crate) fn mod_rs(modules: &BTreeSet<String>) -> TokenStream {
    let modules: Vec<_> = modules.iter().map(|m| format_ident!("{m}")).collect();
    quote! {
        //! The bot's event graphs (D-091).
        //! Generated by RustBot Creator: do not edit, it is overwritten.

        use djs_rs::prelude::*;

        #(mod #modules;)*

        pub(crate) fn register(client: &Client) {
            #(#modules::register(client);)*
        }
    }
}
