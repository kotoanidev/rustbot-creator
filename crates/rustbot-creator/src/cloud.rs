//! The RustBot Cloud client (D-125, D-156–D-174): the HTTP API of `docs/CLOUD_API.md`.
//! The calls block, so they run in the background. The app talks to Cloud only when the
//! user starts something there (D-060), and only when Cloud's address is known.

use std::collections::BTreeMap;
use std::time::Duration;

use uuid::Uuid;

/// The address's variable, until Cloud has its domain (D-163).
const ADDRESS: &str = "RUSTBOT_CLOUD_URL";

/// Cloud's address, or `None` when Cloud is hidden. Only HTTPS, except a local server
/// for development, so a token never travels in the clear.
pub(crate) fn address() -> Option<String> {
    checked_address(&std::env::var(ADDRESS).ok()?)
}

fn checked_address(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('/');
    let local = ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
        .iter()
        .any(|prefix| value.starts_with(prefix));
    (value.starts_with("https://") || local).then(|| value.to_owned())
}

/// Why a call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    /// The session is missing, expired or revoked: sign in again.
    SignedOut,
    /// Cloud said no, with one of its error codes and an English message.
    Api { code: String, message: String },
    /// No answer, or one that makes no sense.
    Network(String),
}

/// A login started in the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Login {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) interval: Duration,
}

/// Where a login stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoginState {
    Pending,
    Done { session: String, name: String },
}

/// A published bot as Cloud sees it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct Bot {
    pub(crate) state: String,
    pub(crate) deployed_at: Option<String>,
    #[serde(default)]
    pub(crate) codegen: String,
    pub(crate) memory_kb: Option<u64>,
    #[serde(default)]
    pub(crate) has_token: bool,
}

/// A memory backup.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct Backup {
    pub(crate) id: String,
    pub(crate) time: String,
    pub(crate) size: u64,
}

/// A log line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct Line {
    pub(crate) stream: String,
    pub(crate) text: String,
}

/// Cloud's error body (D-163).
#[derive(serde::Deserialize)]
struct Failure {
    error: String,
    #[serde(default)]
    message: String,
}

/// Calls to one Cloud, signed in with `session` when there is one.
#[derive(Debug, Clone)]
pub(crate) struct Client {
    base: String,
    session: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    pub(crate) fn new(base: String, session: Option<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            // Cloud closes a connection when it answers before reading a body, e.g. 401,
            // so a pooled one could be dead.
            .max_idle_connections(0)
            .user_agent(concat!("rustbot-creator/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self {
            base,
            session,
            agent,
        }
    }

    /// One call: the status and the body, or the API's error.
    fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Vec<u8>, Error> {
        let url = format!("{}/v1{path}", self.base);
        let auth = self.session.as_ref().map(|s| format!("Bearer {s}"));
        let response = match (method, body) {
            ("GET", _) => with_auth(self.agent.get(&url), auth.as_deref()).call(),
            ("DELETE", _) => with_auth(self.agent.delete(&url), auth.as_deref()).call(),
            (_, body) => {
                let request = match method {
                    "PUT" => self.agent.put(&url),
                    _ => self.agent.post(&url),
                };
                let request = with_auth(request, auth.as_deref());
                match body {
                    Some(body) => request
                        .header("Content-Type", "application/json")
                        .send(body.to_string()),
                    None => request.send_empty(),
                }
            }
        };
        let mut response = response.map_err(|e| Error::Network(e.to_string()))?;
        let status = response.status().as_u16();
        let bytes = response
            .body_mut()
            .with_config()
            .limit(32 << 20)
            .read_to_vec()
            .map_err(|e| Error::Network(e.to_string()))?;
        if (200..300).contains(&status) {
            return Ok(bytes);
        }
        match serde_json::from_slice::<Failure>(&bytes) {
            Ok(failure) if failure.error == "signed_out" => Err(Error::SignedOut),
            Ok(failure) => Err(Error::Api {
                code: failure.error,
                message: failure.message,
            }),
            Err(_) => Err(Error::Network(format!("HTTP {status}"))),
        }
    }

    fn json<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T, Error> {
        let bytes = self.call(method, path, body)?;
        serde_json::from_slice(&bytes).map_err(|e| Error::Network(e.to_string()))
    }

    /// `POST /v1/logins`: the browser's address and how to poll.
    pub(crate) fn start_login(&self) -> Result<Login, Error> {
        #[derive(serde::Deserialize)]
        struct Started {
            id: String,
            url: String,
            interval: u64,
        }
        let started: Started = self.json("POST", "/logins", None)?;
        // The browser opens it, so it must be a web page, not e.g. a file.
        if !started.url.starts_with("https://") {
            return Err(Error::Network(
                "a sign-in address that is not HTTPS".to_owned(),
            ));
        }
        Ok(Login {
            id: started.id,
            url: started.url,
            interval: Duration::from_secs(started.interval.clamp(1, 10)),
        })
    }

    /// `GET /v1/logins/<id>`.
    pub(crate) fn poll_login(&self, id: &str) -> Result<LoginState, Error> {
        #[derive(serde::Deserialize)]
        struct Answer {
            session: Option<String>,
            user: Option<User>,
        }
        #[derive(serde::Deserialize)]
        struct User {
            name: String,
        }
        let answer: Answer = self.json("GET", &format!("/logins/{}", path_part(id)), None)?;
        match (answer.session, answer.user) {
            (Some(session), Some(user)) => Ok(LoginState::Done {
                session,
                name: user.name,
            }),
            _ => Ok(LoginState::Pending),
        }
    }

    /// `GET /v1/me`: the signed-in user's name.
    pub(crate) fn me(&self) -> Result<String, Error> {
        #[derive(serde::Deserialize)]
        struct Me {
            name: String,
        }
        Ok(self.json::<Me>("GET", "/me", None)?.name)
    }

    /// `DELETE /v1/session`.
    pub(crate) fn sign_out(&self) -> Result<(), Error> {
        self.call("DELETE", "/session", None).map(drop)
    }

    /// `GET /v1/bots/<project>`, or `None` when the project is not published.
    pub(crate) fn bot(&self, project: Uuid) -> Result<Option<Bot>, Error> {
        match self.json("GET", &format!("/bots/{project}"), None) {
            Err(Error::Api { code, .. }) if code == "not_found" => Ok(None),
            other => other.map(Some),
        }
    }

    /// `PUT /v1/bots/<project>/token`, only after the user's consent (D-161).
    pub(crate) fn set_token(&self, project: Uuid, token: &str) -> Result<(), Error> {
        let body = serde_json::json!({"token": token.trim()});
        self.call("PUT", &format!("/bots/{project}/token"), Some(&body))
            .map(drop)
    }

    /// `POST /v1/bots/<project>/deploys`: the deploy's id.
    pub(crate) fn deploy(
        &self,
        project: Uuid,
        name: &str,
        package: &str,
        codegen: &str,
        files: &BTreeMap<String, String>,
    ) -> Result<String, Error> {
        #[derive(serde::Deserialize)]
        struct Started {
            deploy: String,
        }
        let body = serde_json::json!({
            "name": name, "package": package, "codegen": codegen, "files": files,
        });
        let started: Started =
            self.json("POST", &format!("/bots/{project}/deploys"), Some(&body))?;
        Ok(started.deploy)
    }

    /// `GET /v1/bots/<project>/deploys/<id>`: its state and the end of its build's log.
    pub(crate) fn deploy_state(&self, project: Uuid, id: &str) -> Result<(String, String), Error> {
        #[derive(serde::Deserialize)]
        struct State {
            state: String,
            #[serde(default)]
            log: String,
        }
        let path = format!("/bots/{project}/deploys/{}", path_part(id));
        let state: State = self.json("GET", &path, None)?;
        Ok((state.state, state.log))
    }

    /// `POST /v1/bots/<project>/start` or `/stop`.
    pub(crate) fn switch(&self, project: Uuid, on: bool) -> Result<(), Error> {
        let verb = if on { "start" } else { "stop" };
        self.call("POST", &format!("/bots/{project}/{verb}"), None)
            .map(drop)
    }

    /// `DELETE /v1/bots/<project>`.
    pub(crate) fn delete(&self, project: Uuid) -> Result<(), Error> {
        self.call("DELETE", &format!("/bots/{project}"), None)
            .map(drop)
    }

    /// `GET /v1/bots/<project>/logs`: the lines after `cursor`, and the next cursor.
    pub(crate) fn logs(
        &self,
        project: Uuid,
        cursor: Option<&str>,
    ) -> Result<(Vec<Line>, String), Error> {
        #[derive(serde::Deserialize)]
        struct Logs {
            lines: Vec<Line>,
            cursor: String,
        }
        let after = cursor.map_or_else(String::new, |c| format!("?after={}", path_part(c)));
        let logs: Logs = self.json("GET", &format!("/bots/{project}/logs{after}"), None)?;
        Ok((logs.lines, logs.cursor))
    }

    /// `GET /v1/bots/<project>/backups`, newest first.
    pub(crate) fn backups(&self, project: Uuid) -> Result<Vec<Backup>, Error> {
        self.json("GET", &format!("/bots/{project}/backups"), None)
    }

    /// `GET /v1/bots/<project>/backups/<id>`: the file.
    pub(crate) fn download(&self, project: Uuid, id: &str) -> Result<Vec<u8>, Error> {
        self.call(
            "GET",
            &format!("/bots/{project}/backups/{}", path_part(id)),
            None,
        )
    }

    /// `POST /v1/bots/<project>/backups/<id>/restore`.
    pub(crate) fn restore(&self, project: Uuid, id: &str) -> Result<(), Error> {
        let path = format!("/bots/{project}/backups/{}/restore", path_part(id));
        self.call("POST", &path, None).map(drop)
    }
}

fn with_auth<B>(request: ureq::RequestBuilder<B>, auth: Option<&str>) -> ureq::RequestBuilder<B> {
    match auth {
        Some(auth) => request.header("Authorization", auth),
        None => request,
    }
}

/// `text` percent-encoded for one part of a URL, since ids come from the server.
fn path_part(text: &str) -> String {
    use std::fmt::Write as _;
    text.bytes().fold(String::new(), |mut out, b| {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader, Read as _, Write as _};
    use std::net::TcpListener;

    use super::{Client, Error, LoginState, path_part};

    /// A one-request-per-connection stand-in for Cloud: each request gets the next
    /// canned answer, and the request lines come back for checking.
    fn cloud(answers: Vec<(u16, &'static str)>) -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (sender, requests) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for (status, body) in answers {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut head = String::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let mut sent = vec![0; length];
                reader.read_exact(&mut sent).unwrap();
                let first = head.lines().next().unwrap_or("").to_owned();
                let auth = head.lines().any(|l| {
                    l.to_ascii_lowercase()
                        .starts_with("authorization: bearer s1")
                });
                sender
                    .send(format!(
                        "{first} auth={auth} {}",
                        String::from_utf8_lossy(&sent)
                    ))
                    .unwrap();
                let mut stream = stream;
                write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (address, requests)
    }

    #[test]
    fn signing_in_polls_until_the_session_comes() {
        let (address, requests) = cloud(vec![
            (
                201,
                r#"{"id":"L1","url":"https://discord.example/a","interval":2,"expires_in":600}"#,
            ),
            (202, r#"{"status":"pending"}"#),
            (
                200,
                r#"{"session":"s1","user":{"id":"111","name":"Alice"}}"#,
            ),
            (200, r#"{"id":"111","name":"Alice","limits":{}}"#),
        ]);
        let client = Client::new(address.clone(), None);
        let login = client.start_login().unwrap();
        assert_eq!(login.url, "https://discord.example/a");
        assert_eq!(client.poll_login(&login.id).unwrap(), LoginState::Pending);
        let done = client.poll_login(&login.id).unwrap();
        assert_eq!(
            done,
            LoginState::Done {
                session: "s1".to_owned(),
                name: "Alice".to_owned()
            }
        );
        let signed = Client::new(address, Some("s1".to_owned()));
        assert_eq!(signed.me().unwrap(), "Alice");
        let seen: Vec<String> = requests.try_iter().collect();
        assert!(seen[0].starts_with("POST /v1/logins HTTP/1.1 auth=false"));
        assert!(seen[1].starts_with("GET /v1/logins/L1 "));
        assert!(seen[3].starts_with("GET /v1/me HTTP/1.1 auth=true"));
    }

    #[test]
    fn errors_and_missing_bots_are_told_apart() {
        let project = uuid::Uuid::nil();
        let (address, requests) = cloud(vec![
            (404, r#"{"error":"not_found","message":"no"}"#),
            (401, r#"{"error":"signed_out","message":"again"}"#),
            (403, r#"{"error":"limit","message":"one bot"}"#),
            (500, "<html>"),
            (202, r#"{"deploy":"D1"}"#),
        ]);
        let client = Client::new(address, Some("s1".to_owned()));
        assert_eq!(client.bot(project).unwrap(), None);
        assert_eq!(client.switch(project, true), Err(Error::SignedOut));
        assert_eq!(
            client.set_token(project, " a.b.c \n"),
            Err(Error::Api {
                code: "limit".to_owned(),
                message: "one bot".to_owned()
            })
        );
        assert!(matches!(client.backups(project), Err(Error::Network(_))));
        let files = [("Cargo.toml".to_owned(), "[package]".to_owned())].into();
        assert_eq!(
            client.deploy(project, "Bot", "bot", "h", &files).unwrap(),
            "D1"
        );
        let seen: Vec<String> = requests.try_iter().collect();
        assert!(seen[2].contains(r#"{"token":"a.b.c"}"#), "{}", seen[2]);
        assert!(
            seen[4].contains(r#""files":{"Cargo.toml":"[package]"}"#),
            "{}",
            seen[4]
        );
    }

    #[test]
    fn only_https_or_a_local_server_is_an_address() {
        let check = super::checked_address;
        assert_eq!(
            check("https://cloud.example.com/"),
            Some("https://cloud.example.com".to_owned())
        );
        assert_eq!(
            check("http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".to_owned())
        );
        assert_eq!(check("http://cloud.example.com"), None);
        assert_eq!(check("file:///etc/passwd"), None);
        assert_eq!(path_part("a/../b?c"), "a%2F..%2Fb%3Fc");
    }
}
