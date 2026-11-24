//! RustBot Cloud in the app (D-125, D-156–D-174): signing in with Discord, "Publish",
//! and the published bot's state, logs and memory backups. All of it is hidden without
//! Cloud's address (D-171), and nothing is asked of Cloud while signed out (D-060).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use gpui_kit::{App, AppContext as _, AsyncWindowContext, Context, Entity, Window};

use crate::cloud::{self, Client, LoginState};
use crate::i18n::{t, t_with};
use crate::install::ask;
use crate::run::{self, Run};
use crate::state::{AppState, Line, Source};

/// How often the published bot's logs are fetched (D-172), and how many of those ticks
/// pass between looks at its state, and at its backups.
const TICK: Duration = Duration::from_secs(2);
const TICKS_PER_CHECK: u32 = 3;
const CHECKS_PER_BACKUPS: u32 = 10;
/// A sign-in waits this long for the browser.
const LOGIN_TIME: Duration = Duration::from_secs(600);

/// RustBot Cloud as last seen.
#[derive(Default)]
pub(crate) struct Published {
    /// Cloud's address, `None` while Cloud is hidden.
    pub(crate) address: Option<String>,
    session: Option<String>,
    /// The signed-in user's name, once known.
    pub(crate) user: Option<String>,
    /// A sign-in waits for the browser.
    pub(crate) signing_in: bool,
    /// The project's published bot, `None` when it is not published or not looked at.
    pub(crate) bot: Option<cloud::Bot>,
    /// Run stopped it and starts it again when it ends (D-173).
    pub(crate) paused: bool,
    /// The graphs have changed since it was published.
    pub(crate) out_of_date: bool,
    /// Its memory backups, newest first (D-174).
    pub(crate) backups: Vec<cloud::Backup>,
    cursor: Option<String>,
}

impl Published {
    fn client(&self) -> Option<Client> {
        Some(Client::new(self.address.clone()?, self.session.clone()))
    }

    pub(crate) fn signed_in(&self) -> bool {
        self.session.is_some()
    }

    pub(crate) fn running(&self) -> bool {
        self.bot
            .as_ref()
            .is_some_and(|bot| matches!(bot.state.as_str(), "running" | "starting"))
    }

    /// The status bar's text, e.g. "Cloud: running · out of date".
    pub(crate) fn label(&self) -> Option<String> {
        let bot = self.bot.as_ref()?;
        let key = match bot.state.as_str() {
            _ if self.paused => "cloud-state-paused",
            "running" => "cloud-state-running",
            "starting" => "cloud-state-starting",
            "building" => "cloud-state-building",
            "failed" => "cloud-state-failed",
            "build_failed" => "cloud-state-build-failed",
            _ => "cloud-state-stopped",
        };
        let label = format!("☁ {}", t(key));
        Some(if self.out_of_date {
            format!("{label} · {}", t("host-out-of-date"))
        } else {
            label
        })
    }

    /// Signed in to `address` as `name`, for tests.
    #[cfg(test)]
    pub(crate) fn signed_in_for_test(address: &str, name: &str) -> Self {
        Self {
            address: Some(address.to_owned()),
            session: Some("session".to_owned()),
            user: Some(name.to_owned()),
            ..Self::default()
        }
    }

    fn forget_session(&mut self) {
        self.session = None;
        self.user = None;
        self.bot = None;
        self.backups.clear();
        self.cursor = None;
    }
}

/// What an error says to the user.
fn describe(error: &cloud::Error) -> String {
    match error {
        cloud::Error::SignedOut => t("cloud-signed-out"),
        cloud::Error::Api { code, message } => match code.as_str() {
            "not_invited" => t("cloud-not-invited"),
            "limit" => t("cloud-limit"),
            "too_large" => t("cloud-too-large"),
            "busy" => t("cloud-busy"),
            "expired" => t("cloud-login-expired"),
            "update_app" => t("cloud-update-app"),
            "no_token" => t("cloud-no-token"),
            _ => message.clone(),
        },
        cloud::Error::Network(error) => t_with("cloud-network", &[("error", error)]),
    }
}

/// Runs a blocking call to Cloud in the background.
async fn call<T: Send + 'static>(
    cx: &mut AsyncWindowContext,
    client: &Client,
    work: impl FnOnce(&Client) -> Result<T, cloud::Error> + Send + 'static,
) -> Result<T, cloud::Error> {
    let client = client.clone();
    cx.background_spawn(async move { work(&client) }).await
}

fn signed_client(state: &AppState) -> Option<Client> {
    state
        .published
        .signed_in()
        .then(|| state.published.client())
        .flatten()
}

/// Watches the published bot while the project is open and the user is signed in: its
/// state every 6 s, its backups every minute and its logs every 2 s (D-172).
pub(crate) fn watch(cx: &mut Context<'_, AppState>) {
    let Some(address) = cloud::address() else {
        return;
    };
    cx.spawn(async move |this, cx| {
        let session = cx
            .background_spawn(async { rustbot_secrets::cloud_session().ok().flatten() })
            .await;
        let stored = this.update(cx, |state, cx| {
            state.published.address = Some(address);
            state.published.session = session;
            cx.notify();
        });
        if stored.is_err() {
            return;
        }
        let mut ticks: u32 = 0;
        loop {
            let Ok(seen) = this.read_with(cx, |state, _| {
                signed_client(state).map(|client| {
                    let p = &state.published;
                    (
                        client,
                        state.project.manifest.id,
                        p.user.is_none(),
                        p.cursor.clone(),
                        p.bot.is_some(),
                    )
                })
            }) else {
                return;
            };
            if let Some((client, project, unnamed, cursor, published)) = seen {
                let check = ticks.is_multiple_of(TICKS_PER_CHECK);
                let backups = check && ticks.is_multiple_of(TICKS_PER_CHECK * CHECKS_PER_BACKUPS);
                let answer = cx
                    .background_spawn(async move {
                        let name = if unnamed { Some(client.me()) } else { None };
                        let bot = check.then(|| client.bot(project));
                        let list = (backups && published).then(|| client.backups(project));
                        let logs = published.then(|| client.logs(project, cursor.as_deref()));
                        (name, bot, list, logs)
                    })
                    .await;
                if this.update(cx, |state, cx| saw(state, answer, cx)).is_err() {
                    return;
                }
            }
            ticks = ticks.wrapping_add(1);
            cx.background_executor().timer(TICK).await;
        }
    })
    .detach();
}

type Answer = (
    Option<Result<String, cloud::Error>>,
    Option<Result<Option<cloud::Bot>, cloud::Error>>,
    Option<Result<Vec<cloud::Backup>, cloud::Error>>,
    Option<Result<(Vec<cloud::Line>, String), cloud::Error>>,
);

fn saw(state: &mut AppState, (name, bot, backups, logs): Answer, cx: &mut Context<'_, AppState>) {
    let results = [
        name.as_ref().map(|r| r.as_ref().map(drop)),
        bot.as_ref().map(|r| r.as_ref().map(drop)),
        backups.as_ref().map(|r| r.as_ref().map(drop)),
        logs.as_ref().map(|r| r.as_ref().map(drop)),
    ];
    if results
        .iter()
        .flatten()
        .any(|r| r == &Err(&cloud::Error::SignedOut))
    {
        state.published.forget_session();
        let _ = rustbot_secrets::delete_cloud_session();
        state.log_error(t("cloud-signed-out"));
        cx.notify();
        return;
    }
    // Other failures, e.g. no network, keep the last answer: the next look may work.
    if let Some(Ok(name)) = name {
        state.published.user = Some(name);
    }
    if let Some(Ok(bot)) = bot {
        if bot.is_none() {
            state.published.backups.clear();
            state.published.cursor = None;
        }
        // Graphs with problems generate nothing; the last answer stands then.
        if let (Some(bot), Ok(output)) = (
            &bot,
            rustbot_codegen::generate(&state.project, &state.registry),
        ) {
            state.published.out_of_date = bot.codegen != crate::host::fingerprint(&output);
        }
        state.published.bot = bot;
    }
    if let Some(Ok(backups)) = backups {
        state.published.backups = backups;
    }
    if let Some(Ok((lines, cursor))) = logs {
        // The first look shows only the last lines, as the hosted journal does.
        let skip = if state.published.cursor.is_none() {
            lines.len().saturating_sub(30)
        } else {
            0
        };
        for line in lines.into_iter().skip(skip) {
            let mut line = Line::output(Source::Bot, line.text, line.stream == "stderr");
            line.text = format!("[{}] {}", t("log-cloud"), line.text);
            state.push(line);
        }
        state.published.cursor = Some(cursor);
    }
    cx.notify();
}

/// "Sign in with Discord" (D-160): the browser signs in, and the app polls for its
/// session, which goes to the keyring.
pub(crate) fn sign_in(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    let published = &state.read(cx).published;
    if published.signing_in || published.signed_in() {
        return;
    }
    let Some(client) = published.client() else {
        return;
    };
    state.update(cx, |state, cx| {
        state.published.signing_in = true;
        cx.notify();
    });
    window
        .spawn(cx, async move |cx| {
            let result = wait_for_login(&client, cx).await;
            state.update(cx, |state, cx| {
                state.published.signing_in = false;
                match result {
                    Ok((session, name)) => {
                        state.published.session = Some(session);
                        state.log(t_with("cloud-signed-in", &[("name", &name)]));
                        state.published.user = Some(name);
                    }
                    Err(e) => state.log_error(e),
                }
                cx.notify();
            });
        })
        .detach();
}

async fn wait_for_login(
    client: &Client,
    cx: &mut AsyncWindowContext,
) -> Result<(String, String), String> {
    let login = call(cx, client, Client::start_login)
        .await
        .map_err(|e| describe(&e))?;
    let url = login.url.clone();
    let _ = cx.update(|_, cx| cx.open_url(&url));
    let deadline = Instant::now() + LOGIN_TIME;
    loop {
        cx.background_executor().timer(login.interval).await;
        let id = login.id.clone();
        match call(cx, client, move |c| c.poll_login(&id)).await {
            Ok(LoginState::Pending) if Instant::now() < deadline => {}
            Ok(LoginState::Pending) => return Err(t("cloud-login-expired")),
            Ok(LoginState::Done { session, name }) => {
                let kept = session.clone();
                cx.background_spawn(async move { rustbot_secrets::set_cloud_session(&kept) })
                    .await
                    .map_err(|e| t_with("error-keyring", &[("error", &e.to_string())]))?;
                return Ok((session, name));
            }
            Err(e) => return Err(describe(&e)),
        }
    }
}

/// "Sign out": the session is revoked and forgotten. Published bots keep running.
pub(crate) fn sign_out(state: &Entity<AppState>, cx: &mut App) {
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let state = state.clone();
    cx.spawn(async move |cx| {
        cx.background_spawn(async move {
            let _ = client.sign_out();
            rustbot_secrets::delete_cloud_session()
        })
        .await
        .ok();
        state.update(cx, |state, cx| {
            state.published.forget_session();
            state.log(t("cloud-signed-out-log"));
            cx.notify();
        });
    })
    .detach();
}

/// "Publish" and "Update": the sources go to Cloud, which builds and runs them (D-159).
/// The first time asks to send the token (D-161), and local hosting goes first (D-173).
pub(crate) fn publish(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    if !matches!(state.read(cx).run, Run::Stopped) {
        return;
    }
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let first = state
        .read(cx)
        .published
        .bot
        .as_ref()
        .is_none_or(|bot| !bot.has_token);
    let hosted = state.read(cx).hosted.is_hosted();
    window
        .spawn(cx, async move |cx| {
            if hosted {
                let (title, detail) = (t("cloud-unhost-title"), t("cloud-unhost-detail"));
                if !ask(cx, &title, &detail, &t("cloud-unhost")).await {
                    return;
                }
            }
            if first {
                let (title, detail) = (t("cloud-token-title"), t("cloud-token-detail"));
                if !ask(cx, &title, &detail, &t("cloud-token-send")).await {
                    return;
                }
            }
            if !run::begin(&state, cx, &t("cloud-publishing")) {
                return;
            }
            let result = deploy(&state, &client, first, hosted, cx).await;
            run::finish(&state, cx, result.map(|()| t("cloud-published")));
        })
        .detach();
}

async fn deploy(
    state: &Entity<AppState>,
    client: &Client,
    first: bool,
    hosted: bool,
    cx: &mut AsyncWindowContext,
) -> Result<(), String> {
    if hosted {
        crate::host::unhost(state, cx).await?;
    }
    let output = run::generate(state, cx)?;
    let codegen = crate::host::fingerprint(&output);
    let project = state.read_with(cx, |state, _| state.project.clone());
    let id = project.manifest.id;
    let mut files = BTreeMap::new();
    for (path, bytes) in crate::export::source_files(&project, output) {
        let name: Vec<String> = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let text = String::from_utf8(bytes)
            .map_err(|_| t_with("cloud-not-text", &[("file", &path.display().to_string())]))?;
        files.insert(name.join("/"), text);
    }
    if first {
        let token = cx
            .background_spawn(async move { rustbot_secrets::bot_token(id) })
            .await
            .map_err(|e| t_with("error-keyring", &[("error", &e.to_string())]))?
            .ok_or_else(|| t("run-no-token"))?;
        call(cx, client, move |c| c.set_token(id, &token))
            .await
            .map_err(|e| describe(&e))?;
    }
    let (name, package) = (project.manifest.bot.name, project.manifest.bot.package);
    let deploy = call(cx, client, move |c| {
        c.deploy(id, &name, &package, &codegen, &files)
    })
    .await
    .map_err(|e| describe(&e))?;
    run::busy(state, cx, &t("cloud-building"));
    let (state_now, log) = loop {
        cx.background_executor().timer(TICK).await;
        let deploy = deploy.clone();
        let (now, log) = call(cx, client, move |c| c.deploy_state(id, &deploy))
            .await
            .map_err(|e| describe(&e))?;
        if now == "done" || now == "failed" {
            break (now, log);
        }
    };
    if state_now == "failed" {
        state.update(cx, |state, cx| {
            for line in log.lines() {
                state.push(Line::output(Source::Build, line.to_owned(), true));
            }
            cx.notify();
        });
        return Err(t("cloud-build-failed"));
    }
    // A running bot already moved onto the new build; a stopped one starts now.
    let bot = call(cx, client, move |c| c.bot(id))
        .await
        .map_err(|e| describe(&e))?;
    if bot
        .as_ref()
        .is_some_and(|bot| !matches!(bot.state.as_str(), "running" | "starting"))
    {
        call(cx, client, move |c| c.switch(id, true))
            .await
            .map_err(|e| describe(&e))?;
    }
    refresh(state, client, cx).await;
    Ok(())
}

/// Looks at the bot and its backups now, after a change.
async fn refresh(state: &Entity<AppState>, client: &Client, cx: &mut AsyncWindowContext) {
    let id = state.read_with(cx, |state, _| state.project.manifest.id);
    let answer = call(cx, client, move |c| Ok((c.bot(id), c.backups(id)))).await;
    if let Ok((bot, backups)) = answer {
        state.update(cx, |state, cx| {
            saw(state, (None, Some(bot), Some(backups), None), cx);
        });
    }
}

/// Start or Stop in the bot settings.
pub(crate) fn switch(state: Entity<AppState>, on: bool, window: &mut Window, cx: &mut App) {
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let id = state.read(cx).project.manifest.id;
    window
        .spawn(cx, async move |cx| {
            if let Err(e) = call(cx, &client, move |c| c.switch(id, on)).await {
                state.update(cx, |state, _| state.log_error(describe(&e)));
            }
            refresh(&state, &client, cx).await;
        })
        .detach();
}

/// "Delete…": the bot, its token, its memory and its backups leave Cloud.
pub(crate) fn delete(state: Entity<AppState>, window: &mut Window, cx: &mut App) {
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let id = state.read(cx).project.manifest.id;
    window
        .spawn(cx, async move |cx| {
            if !delete_now(&state, &client, id, true, cx).await {
                return;
            }
            refresh(&state, &client, cx).await;
        })
        .detach();
}

/// Deletes the bot from Cloud, after asking when `confirm`. Whether it was deleted.
pub(crate) async fn delete_now(
    state: &Entity<AppState>,
    client: &Client,
    id: uuid::Uuid,
    confirm: bool,
    cx: &mut AsyncWindowContext,
) -> bool {
    let (title, detail) = (t("cloud-delete-title"), t("cloud-delete-detail"));
    if confirm && !ask(cx, &title, &detail, &t("cloud-delete")).await {
        return false;
    }
    let result = call(cx, client, move |c| c.delete(id)).await;
    state.update(cx, |state, cx| {
        let deleted = match result {
            Ok(()) => {
                state.log(t("cloud-deleted"));
                state.published.bot = None;
                state.published.backups.clear();
                true
            }
            Err(e) => {
                state.log_error(describe(&e));
                false
            }
        };
        cx.notify();
        deleted
    })
}

/// Hosting locally first deletes the Cloud copy, after asking (D-173). Whether hosting may
/// go on.
pub(crate) async fn leave_cloud(state: &Entity<AppState>, cx: &mut AsyncWindowContext) -> bool {
    let Some((client, id)) = state.read_with(cx, |state, _| {
        let client = signed_client(state)?;
        state.published.bot.as_ref()?;
        Some((client, state.project.manifest.id))
    }) else {
        return true;
    };
    let (title, detail) = (t("cloud-leave-title"), t("cloud-leave-detail"));
    if !ask(cx, &title, &detail, &t("cloud-leave")).await {
        return false;
    }
    delete_now(state, &client, id, false, cx).await
}

/// "Restore…" a backup (D-174): Cloud backs up the current memory first.
pub(crate) fn restore(state: Entity<AppState>, backup: String, window: &mut Window, cx: &mut App) {
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let id = state.read(cx).project.manifest.id;
    window
        .spawn(cx, async move |cx| {
            let (title, detail) = (t("cloud-restore-title"), t("cloud-restore-detail"));
            if !ask(cx, &title, &detail, &t("cloud-restore")).await {
                return;
            }
            let result = call(cx, &client, move |c| c.restore(id, &backup)).await;
            state.update(cx, |state, _| match result {
                Ok(()) => state.log(t("cloud-restored")),
                Err(e) => state.log_error(describe(&e)),
            });
            refresh(&state, &client, cx).await;
        })
        .detach();
}

/// "Download" a backup to a file the user picks (D-174).
pub(crate) fn download(state: Entity<AppState>, backup: String, window: &mut Window, cx: &mut App) {
    let Some(client) = signed_client(state.read(cx)) else {
        return;
    };
    let id = state.read(cx).project.manifest.id;
    window
        .spawn(cx, async move |cx| {
            let home = crate::settings::home();
            let Ok(path) = cx.update(|_, cx| cx.prompt_for_new_path(&home, Some("data.json")))
            else {
                return;
            };
            let Some(path) = path.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let result = call(cx, &client, move |c| c.download(id, &backup))
                .await
                .map_err(|e| describe(&e));
            let shown = path.display().to_string();
            let written = match result {
                Ok(bytes) => cx
                    .background_spawn(async move { std::fs::write(&path, bytes) })
                    .await
                    .map_err(|e| e.to_string()),
                Err(e) => Err(e),
            };
            state.update(cx, |state, _| match written {
                Ok(()) => state.log(t_with("cloud-downloaded", &[("path", &shown)])),
                Err(e) => state.log_error(e),
            });
        })
        .detach();
}

/// Stops the Cloud bot while Run tests this one (D-173).
pub(crate) async fn pause(state: &Entity<AppState>, cx: &mut AsyncWindowContext) {
    let Some((client, id)) = state.read_with(cx, |state, _| {
        state
            .published
            .running()
            .then(|| signed_client(state).map(|c| (c, state.project.manifest.id)))
            .flatten()
    }) else {
        return;
    };
    let stopped = call(cx, &client, move |c| c.switch(id, false)).await;
    state.update(cx, |state, cx| {
        match stopped {
            Ok(()) => {
                state.published.paused = true;
                state.log(t("cloud-paused-log"));
            }
            Err(e) => state.log_error(describe(&e)),
        }
        cx.notify();
    });
}

/// Starts the Cloud bot again after Run, if Run paused it.
pub(crate) async fn resume(state: &Entity<AppState>, cx: &mut AsyncWindowContext) {
    let Some((client, id)) = state.read_with(cx, |state, _| {
        state
            .published
            .paused
            .then(|| signed_client(state).map(|c| (c, state.project.manifest.id)))
            .flatten()
    }) else {
        return;
    };
    let started = call(cx, &client, move |c| c.switch(id, true)).await;
    state.update(cx, |state, cx| {
        state.published.paused = false;
        match started {
            Ok(()) => state.log(t("cloud-resumed-log")),
            Err(e) => state.log_error(describe(&e)),
        }
        cx.notify();
    });
}

/// When the app quits during Run: the Cloud bot Run paused starts again.
pub(crate) fn resume_on_quit(state: &mut AppState) {
    if !state.published.paused {
        return;
    }
    if let Some(client) = signed_client(state) {
        let _ = client.switch(state.project.manifest.id, true);
    }
    state.published.paused = false;
}

#[cfg(test)]
mod tests {
    use super::{Published, describe};
    use crate::cloud::{Bot, Error};
    use crate::i18n::t;

    fn bot(state: &str) -> Bot {
        Bot {
            state: state.to_owned(),
            deployed_at: None,
            codegen: String::new(),
            memory_kb: None,
            has_token: true,
        }
    }

    #[test]
    fn the_label_tells_the_state_and_an_old_build() {
        assert_eq!(Published::default().label(), None);
        let running = Published {
            bot: Some(bot("running")),
            out_of_date: true,
            ..Published::default()
        };
        let label = running.label().unwrap();
        assert!(label.starts_with("☁ "));
        assert!(label.contains(&t("cloud-state-running")));
        assert!(label.contains(&t("host-out-of-date")));
        assert!(running.running());
        let paused = Published {
            bot: Some(bot("stopped")),
            paused: true,
            ..Published::default()
        };
        assert!(paused.label().unwrap().contains(&t("cloud-state-paused")));
        assert!(!paused.running());
    }

    #[test]
    fn known_errors_are_told_in_the_app_s_words() {
        let api = |code: &str| Error::Api {
            code: code.to_owned(),
            message: "server text".to_owned(),
        };
        assert_eq!(describe(&api("not_invited")), t("cloud-not-invited"));
        assert_eq!(describe(&api("something_new")), "server text");
        assert_eq!(describe(&Error::SignedOut), t("cloud-signed-out"));
        assert!(describe(&Error::Network("down".to_owned())).contains("down"));
    }
}
