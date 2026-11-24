//! The Developer Portal guide (D-129, D-131–D-133). Each step says what to do on Discord's
//! site, opens the right page and is checked: the token (kept in the keyring only once
//! Discord knows it), the privileged intents the project needs, the invite with the
//! permissions its nodes need, and the test server, picked from the bot's servers.

use std::collections::BTreeSet;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, h_flex, v_flex};
use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, FontWeight, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, div, px,
};

use crate::discord::{self, Bot, Server};
use crate::i18n::{t, t_with};
use crate::state::AppState;

/// The guide is closed.
pub(crate) struct Close;

/// A check against Discord.
enum Check<T> {
    Idle,
    Busy,
    Done(T),
    Failed(String),
}

pub(crate) struct Guide {
    state: Entity<AppState>,
    token: Entity<InputState>,
    bot: Check<Bot>,
    servers: Check<Vec<Server>>,
    /// The test server's name, once one is picked here.
    picked: Option<String>,
    needed: BTreeSet<&'static str>,
    permissions: u64,
    _enter: Subscription,
}

impl EventEmitter<Close> for Guide {}

/// The name the portal gives a privileged intent's switch.
fn switch_name(intent: &str) -> &'static str {
    match intent {
        "GUILD_MEMBERS" => "Server Members Intent",
        "MESSAGE_CONTENT" => "Message Content Intent",
        _ => "Presence Intent",
    }
}

fn message(error: discord::Error) -> String {
    match error {
        discord::Error::Token => t("guide-token-wrong"),
        discord::Error::Network(error) => t_with("guide-network", &[("error", &error)]),
    }
}

impl Guide {
    pub(crate) fn new(
        state: Entity<AppState>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder(t("bot-token-placeholder"))
        });
        let enter = cx.subscribe_in(&token, window, |this, _, event, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                this.check_typed(window, cx);
            }
        });
        let (needed, permissions) = {
            let state = state.read(cx);
            (
                discord::needed_intents(&state.project, &state.registry),
                discord::permissions(&state.project, &state.registry),
            )
        };
        let mut guide = Self {
            state,
            token,
            bot: Check::Idle,
            servers: Check::Idle,
            picked: None,
            needed,
            permissions,
            _enter: enter,
        };
        // A token saved before is checked at once.
        guide.check(None, cx);
        guide
    }

    /// Checks the typed token, then keeps it in the keyring and clears the field.
    fn check_typed(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let typed = self.token.read(cx).value().trim().to_owned();
        if typed.is_empty() {
            return;
        }
        self.token
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.check(Some(typed), cx);
    }

    /// Asks Discord who the bot of `typed`, or of the saved token, is. A typed token is
    /// saved only when Discord knows it.
    fn check(&mut self, typed: Option<String>, cx: &mut Context<'_, Self>) {
        self.bot = Check::Busy;
        let project = self.state.read(cx).project.manifest.id;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let (token, save) = match typed {
                        Some(token) => (token, true),
                        None => match rustbot_secrets::bot_token(project) {
                            Ok(Some(token)) => (token, false),
                            Ok(None) => return None,
                            Err(error) => return Some(Err(error.to_string())),
                        },
                    };
                    let bot = discord::bot(&token).map_err(message);
                    if save
                        && bot.is_ok()
                        && let Err(error) = rustbot_secrets::set_bot_token(project, &token)
                    {
                        return Some(Err(error.to_string()));
                    }
                    Some(bot)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.bot = match result {
                    None => Check::Idle,
                    Some(Ok(bot)) => Check::Done(bot),
                    Some(Err(error)) => Check::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Lists the servers the bot is on (D-132).
    fn list_servers(&mut self, cx: &mut Context<'_, Self>) {
        self.servers = Check::Busy;
        let project = self.state.read(cx).project.manifest.id;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let token = rustbot_secrets::bot_token(project)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| t("guide-token-first"))?;
                    discord::servers(&token).map_err(message)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.servers = match result {
                    Ok(servers) => Check::Done(servers),
                    Err(error) => Check::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn pick(&mut self, server: &Server, cx: &mut Context<'_, Self>) {
        let state = self.state.read(cx);
        let Some(dir) = state
            .toolchain
            .as_ref()
            .map(|toolchain| toolchain.bot_dir(&state.project))
        else {
            return;
        };
        match rustbot_runner::set_test_server(&dir, Some(&server.id)) {
            Ok(()) => self.picked = Some(server.name.clone()),
            Err(error) => self.servers = Check::Failed(error.to_string()),
        }
        cx.notify();
    }

    fn token_step(&self, cx: &mut Context<'_, Self>) -> gpui_kit::Div {
        let theme = cx.theme();
        let status = match &self.bot {
            Check::Idle => None,
            Check::Busy => Some((t("guide-checking"), false)),
            Check::Done(bot) => Some((t_with("guide-token-ok", &[("name", &bot.name)]), false)),
            Check::Failed(error) => Some((error.clone(), true)),
        };
        step(2, &t("guide-token"), t("guide-token-about"))
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(div().w(px(360.)).child(Input::new(&self.token)))
                    .child(
                        Button::new("guide-check")
                            .primary()
                            .label(t("guide-check"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.check_typed(window, cx);
                            })),
                    ),
            )
            .children(status.map(|(text, error)| {
                div()
                    .text_sm()
                    .text_color(if error {
                        theme.danger
                    } else {
                        theme.muted_foreground
                    })
                    .child(text)
            }))
    }

    fn intents_step(&self, cx: &mut Context<'_, Self>) -> Option<gpui_kit::Div> {
        if self.needed.is_empty() {
            return None;
        }
        let on = match &self.bot {
            Check::Done(bot) => Some(&bot.intents),
            _ => None,
        };
        let lines = self.needed.iter().map(|intent| {
            let mark = match on {
                Some(on) if on.contains(intent) => "✓",
                Some(_) => "✗",
                None => "•",
            };
            format!("{mark} {}", switch_name(intent))
        });
        let application = match &self.bot {
            Check::Done(bot) => Some(bot.application),
            _ => None,
        };
        Some(
            step(3, &t("guide-intents"), t("guide-intents-about"))
                .children(lines)
                .child(
                    h_flex()
                        .gap(px(8.))
                        .child(
                            Button::new("guide-bot-page")
                                .label(t("guide-bot-page"))
                                .disabled(application.is_none())
                                .on_click(move |_, _, cx| {
                                    if let Some(application) = application {
                                        cx.open_url(&discord::bot_page(application));
                                    }
                                }),
                        )
                        .child(
                            Button::new("guide-check-again")
                                .label(t("guide-check-again"))
                                .on_click(cx.listener(|this, _, _, cx| this.check(None, cx))),
                        ),
                ),
        )
    }

    fn server_step(&self, number: usize, cx: &mut Context<'_, Self>) -> gpui_kit::Div {
        let theme = cx.theme();
        let application = match &self.bot {
            Check::Done(bot) => Some(bot.application),
            _ => None,
        };
        let permissions = self.permissions;
        let mut body = step(number, &t("guide-invite"), t("guide-invite-about")).child(
            h_flex()
                .gap(px(8.))
                .child(
                    Button::new("guide-invite")
                        .primary()
                        .label(t("guide-invite-button"))
                        .disabled(application.is_none())
                        .on_click(move |_, _, cx| {
                            if let Some(application) = application {
                                cx.open_url(&discord::invite_link(application, permissions));
                            }
                        }),
                )
                .child(
                    Button::new("guide-servers")
                        .label(t("guide-servers"))
                        .disabled(application.is_none())
                        .on_click(cx.listener(|this, _, _, cx| this.list_servers(cx))),
                ),
        );
        match &self.servers {
            Check::Idle => {}
            Check::Busy => body = body.child(t("guide-checking")),
            Check::Done(servers) if servers.is_empty() => {
                body = body.child(t("guide-no-servers"));
            }
            Check::Done(servers) => {
                body = body.child(h_flex().flex_wrap().gap(px(8.)).children(
                    servers.iter().enumerate().map(|(ix, server)| {
                        let server = server.clone();
                        Button::new(("guide-server", ix))
                            .label(server.name.clone())
                            .on_click(cx.listener(move |this, _, _, cx| this.pick(&server, cx)))
                    }),
                ));
            }
            Check::Failed(error) => {
                body = body.child(div().text_color(theme.danger).child(error.clone()));
            }
        }
        body.children(
            self.picked
                .as_ref()
                .map(|name| t_with("guide-server-ok", &[("name", name)])),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(t("guide-server-hint")),
        )
    }
}

/// A numbered step with its title and what to do.
fn step(number: usize, title: &str, about: String) -> gpui_kit::Div {
    v_flex()
        .gap(px(8.))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(format!("{number}. {title}")),
        )
        .child(about)
}

impl Render for Guide {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let intents = self.intents_step(cx);
        let invite_number = if intents.is_some() { 4 } else { 3 };
        let token = self.token_step(cx);
        let server = self.server_step(invite_number, cx);
        let theme = cx.theme();
        v_flex()
            .id("guide")
            .size_full()
            .items_center()
            .overflow_y_scroll()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .w(px(640.))
                    .py(px(32.))
                    .gap(px(20.))
                    .child(div().text_2xl().child(t("guide-title")))
                    .child(
                        step(1, &t("guide-app"), t("guide-app-about")).child(
                            h_flex().child(
                                Button::new("guide-portal")
                                    .label(t("guide-portal"))
                                    .on_click(|_, _, cx| cx.open_url(discord::PORTAL)),
                            ),
                        ),
                    )
                    .child(token)
                    .children(intents)
                    .child(server)
                    .child(
                        h_flex().justify_end().child(
                            Button::new("guide-done")
                                .primary()
                                .label(t("wizard-finish"))
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(Close))),
                        ),
                    ),
            )
    }
}
