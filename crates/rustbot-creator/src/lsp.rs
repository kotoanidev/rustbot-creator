//! Code help from rust-analyzer (D-178): the app's own small LSP client over stdio, and
//! completion, hover and go to definition for the code editor through it.
//!
//! rust-analyzer works on a copy of the project in the app's cache, laid out as the source
//! export is (D-175). Its `Cargo.toml` at the top makes `generated/` and `src/` one
//! package, so the `#[path]` module reaches the user's code. The copy holds only files the
//! app makes and the user's own code, so a project from elsewhere cannot run a program
//! through a `.cargo/config.toml` or a `rust-analyzer.toml` of its own (D-064).

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::Duration;

use gpui_kit::component::input::{
    CompletionProvider, DefinitionProvider, HoverProvider, Rope, RopeExt as _,
};
use gpui_kit::{App, AppContext as _, BackgroundExecutor, Entity, PromptLevel, Task, Window};
use lsp_types::{
    CompletionContext, CompletionResponse, GotoDefinitionResponse, Hover, Location, LocationLink,
};
use serde_json::{Value, json};

use crate::i18n::{t, t_with};
use crate::state::{AppState, CodeHelp, OpenCode};

/// The largest message rust-analyzer may send, so a broken one cannot fill the memory.
const MESSAGE_LIMIT: usize = 64 * 1024 * 1024;
/// How long a request waits for its answer.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(30);
/// How long starting may take: rust-analyzer answers `initialize` before it loads the
/// project, so this is only the program starting.
const START_TIMEOUT: Duration = Duration::from_secs(60);
/// How often an answer and new diagnostics are looked for.
const POLL: Duration = Duration::from_millis(20);
const DIAGNOSTICS_POLL: Duration = Duration::from_millis(300);

type Answer = Result<Value, String>;

/// Code help for the open project, kept in [`AppState`].
#[derive(Default)]
pub(crate) struct Analyzer {
    pub(crate) server: Option<Arc<Server>>,
    /// Starting was tried or asked about, so a new code tab does not ask again.
    asked: bool,
    _poll: Option<Task<()>>,
}

/// A running rust-analyzer.
pub(crate) struct Server {
    stdin: Arc<Mutex<ChildStdin>>,
    child: Mutex<Child>,
    next_id: AtomicI64,
    pending: Arc<Mutex<HashMap<i64, mpsc::Sender<Answer>>>>,
    /// The newest diagnostics by file of the project, `src/…`.
    diagnostics: Arc<Mutex<HashMap<PathBuf, Vec<lsp_types::Diagnostic>>>>,
    /// New diagnostics came since the last look.
    fresh: Arc<AtomicBool>,
    /// The copy rust-analyzer works on.
    copy: PathBuf,
    /// What the copy holds, as last written.
    written: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    /// The open files, with the version and the text last sent.
    open: Mutex<HashMap<PathBuf, (i32, String)>>,
}

impl Server {
    /// Writes `files` to a fresh copy at `copy`, starts `command` (rust-analyzer) on it
    /// and waits until it answers.
    pub(crate) fn start(
        mut command: Command,
        copy: &Path,
        files: BTreeMap<PathBuf, Vec<u8>>,
    ) -> Result<Arc<Self>, String> {
        let failed = |e: io::Error| e.to_string();
        if fs::symlink_metadata(copy).is_ok() {
            fs::remove_dir_all(copy).map_err(failed)?;
        }
        fs::create_dir_all(copy).map_err(failed)?;
        // rust-analyzer names files by their real path.
        #[cfg(unix)]
        let copy = fs::canonicalize(copy).map_err(failed)?;
        #[cfg(not(unix))]
        let copy = copy.to_owned();
        for (path, bytes) in &files {
            write_file(&copy.join(path), bytes).map_err(failed)?;
        }
        let mut child = command
            .current_dir(&copy)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(failed)?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            return Err("rust-analyzer has no stdio".to_owned());
        };
        let server = Arc::new(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            child: Mutex::new(child),
            next_id: AtomicI64::new(1),
            pending: Arc::default(),
            diagnostics: Arc::default(),
            fresh: Arc::default(),
            copy,
            written: Mutex::new(files),
            open: Mutex::default(),
        });
        server.listen(stdout);
        let root = file_uri(&server.copy);
        let started = server.request(
            "initialize",
            &json!({
                "processId": std::process::id(),
                "rootUri": root,
                "workspaceFolders": [{ "uri": root, "name": "bot" }],
                "capabilities": {
                    // The editor counts columns in characters.
                    "general": { "positionEncodings": ["utf-32"] },
                    "textDocument": {
                        "synchronization": { "didSave": true },
                        "completion": { "completionItem": { "snippetSupport": false } },
                        "hover": { "contentFormat": ["markdown", "plaintext"] },
                        "definition": { "linkSupport": true },
                        "publishDiagnostics": {},
                    },
                },
                "initializationOptions": { "files": { "watcher": "server" } },
            }),
        );
        match started.recv_timeout(START_TIMEOUT) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err("rust-analyzer did not start".to_owned()),
        }
        server.notify("initialized", &json!({}));
        Ok(server)
    }

    /// Reads what rust-analyzer sends: answers, diagnostics, and requests, which get an
    /// empty answer as the client offers none.
    fn listen(&self, stdout: ChildStdout) {
        let (stdin, pending) = (self.stdin.clone(), self.pending.clone());
        let (diagnostics, fresh) = (self.diagnostics.clone(), self.fresh.clone());
        let copy = self.copy.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(message)) = read_message(&mut reader) {
                let id = message.get("id");
                match message.get("method").and_then(Value::as_str) {
                    Some(_) if id.is_some() => {
                        let answer = json!({ "jsonrpc": "2.0", "id": id, "result": null });
                        let _ = write_message(&mut *lock(&stdin), &answer);
                    }
                    Some("textDocument/publishDiagnostics") => {
                        let params = message.get("params").cloned().unwrap_or_default();
                        let Ok(params) =
                            serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(params)
                        else {
                            continue;
                        };
                        if let Some(path) = in_copy(&copy, params.uri.as_str()) {
                            lock(&diagnostics).insert(path, params.diagnostics);
                            fresh.store(true, Ordering::Relaxed);
                        }
                    }
                    Some(_) => {}
                    None => {
                        let Some(sender) = id
                            .and_then(Value::as_i64)
                            .and_then(|id| lock(&pending).remove(&id))
                        else {
                            continue;
                        };
                        let answer = match message.get("error") {
                            Some(error) => Err(error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("rust-analyzer failed")
                                .to_owned()),
                            None => Ok(message.get("result").cloned().unwrap_or_default()),
                        };
                        let _ = sender.send(answer);
                    }
                }
            }
            // rust-analyzer stopped, so nothing waits for an answer any longer.
            lock(&pending).clear();
        });
    }

    fn request(&self, method: &str, params: &Value) -> mpsc::Receiver<Answer> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, answer) = mpsc::channel();
        lock(&self.pending).insert(id, sender);
        let message = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if write_message(&mut *lock(&self.stdin), &message).is_err() {
            lock(&self.pending).remove(&id);
        }
        answer
    }

    fn notify(&self, method: &str, params: &Value) {
        let message = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        let _ = write_message(&mut *lock(&self.stdin), &message);
    }

    fn uri(&self, path: &Path) -> String {
        file_uri(&self.copy.join(path))
    }

    /// The project file, `src/…` or `generated/…`, that a URI of the copy names.
    pub(crate) fn project_path(&self, uri: &str) -> Option<PathBuf> {
        in_copy(&self.copy, uri)
    }

    /// Opens `path` of the project with `text`, so rust-analyzer reads it from the editor.
    pub(crate) fn open(&self, path: &Path, text: &str) {
        let language = if path.extension().is_some_and(|e| e == "toml") {
            "toml"
        } else {
            "rust"
        };
        lock(&self.open).insert(path.to_owned(), (1, text.to_owned()));
        self.notify(
            "textDocument/didOpen",
            &json!({ "textDocument": {
                "uri": self.uri(path), "languageId": language, "version": 1, "text": text,
            } }),
        );
    }

    /// Sends the text of open `path` when it changed since it was last sent.
    pub(crate) fn change(&self, path: &Path, text: &str) {
        let version = {
            let mut open = lock(&self.open);
            let Some((version, sent)) = open.get_mut(path) else {
                return;
            };
            if sent == text {
                return;
            }
            *version += 1;
            text.clone_into(sent);
            *version
        };
        self.notify(
            "textDocument/didChange",
            &json!({
                "textDocument": { "uri": self.uri(path), "version": version },
                "contentChanges": [{ "text": text }],
            }),
        );
    }

    /// `path` was saved as `text`: the copy follows, and rust-analyzer checks again.
    pub(crate) fn saved(&self, path: &Path, text: &str) {
        if write_file(&self.copy.join(path), text.as_bytes()).is_ok() {
            lock(&self.written).insert(path.to_owned(), text.as_bytes().to_vec());
        }
        let uri = self.uri(path);
        self.notify(
            "textDocument/didSave",
            &json!({ "textDocument": { "uri": uri } }),
        );
    }

    /// The copy follows the project, whose files are `files` now, and rust-analyzer
    /// checks again when one changed.
    pub(crate) fn sync(&self, files: BTreeMap<PathBuf, Vec<u8>>) {
        let mut written = lock(&self.written);
        let mut changed = None;
        for gone in written.keys().filter(|path| !files.contains_key(*path)) {
            let _ = fs::remove_file(self.copy.join(gone));
            changed = Some(gone.clone());
        }
        for (path, bytes) in &files {
            if written.get(path) != Some(bytes) {
                let _ = write_file(&self.copy.join(path), bytes);
                changed = Some(path.clone());
            }
        }
        *written = files;
        drop(written);
        if let Some(path) = changed {
            let uri = self.uri(&path);
            self.notify(
                "textDocument/didSave",
                &json!({ "textDocument": { "uri": uri } }),
            );
        }
    }

    /// Whether new diagnostics came since the last call.
    fn take_fresh(&self) -> bool {
        self.fresh.swap(false, Ordering::Relaxed)
    }

    /// The newest diagnostics of `path` in the project.
    pub(crate) fn diagnostics(&self, path: &Path) -> Vec<lsp_types::Diagnostic> {
        lock(&self.diagnostics)
            .get(path)
            .cloned()
            .unwrap_or_default()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let child = self.child.get_mut().unwrap_or_else(PoisonError::into_inner);
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, bytes)
}

/// Writes one LSP message: a `Content-Length` header, then the JSON.
fn write_message(out: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = message.to_string();
    write!(out, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    out.flush()
}

/// Reads one LSP message, or `None` at the end of the stream.
fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let length = length
        .filter(|length| *length <= MESSAGE_LIMIT)
        .ok_or_else(|| io::Error::other("a message without a usable Content-Length"))?;
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(io::Error::other)
}

/// A path's text for comparing: `/` between names, and a Windows drive letter in lower
/// case, as rust-analyzer writes it.
fn comparable(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('\\', "/");
    if text.as_bytes().get(1) == Some(&b':') {
        text[..1].make_ascii_lowercase();
    }
    text
}

/// The `file://` URI of an absolute path.
fn file_uri(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from("file://");
    if !path.starts_with('/') {
        uri.push('/');
    }
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 15)]));
        }
    }
    uri
}

/// The path a `file://` URI names.
fn file_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?.as_bytes();
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut at = 0;
    while at < encoded.len() {
        if encoded[at] == b'%' {
            let hex = std::str::from_utf8(encoded.get(at + 1..at + 3)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            bytes.push(encoded[at]);
            at += 1;
        }
    }
    let text = String::from_utf8(bytes).ok()?;
    // `/c:/…` names a Windows drive.
    let text = match text.as_bytes() {
        [b'/', letter, b':', ..] if letter.is_ascii_alphabetic() => text[1..].to_owned(),
        _ => text,
    };
    Some(PathBuf::from(text))
}

/// The path inside `copy` that `uri` names. rustc names the user's code through the
/// `#[path]`, as `generated/src/../../src/…`.
fn in_copy(copy: &Path, uri: &str) -> Option<PathBuf> {
    let path = comparable(&file_path(uri)?);
    let rest = path.strip_prefix(&comparable(copy))?.strip_prefix('/')?;
    let mut inside = PathBuf::new();
    for part in Path::new(rest).components() {
        match part {
            std::path::Component::Normal(name) => inside.push(name),
            std::path::Component::ParentDir if inside.pop() => {}
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(inside)
}

/// The answer to a request, without holding a thread while rust-analyzer works.
async fn wait(
    executor: BackgroundExecutor,
    answer: mpsc::Receiver<Answer>,
) -> anyhow::Result<Value> {
    let mut waited = Duration::ZERO;
    loop {
        match answer.try_recv() {
            Ok(answer) => return answer.map_err(anyhow::Error::msg),
            Err(mpsc::TryRecvError::Disconnected) => anyhow::bail!("rust-analyzer stopped"),
            Err(mpsc::TryRecvError::Empty) if waited >= ANSWER_TIMEOUT => {
                anyhow::bail!("rust-analyzer did not answer")
            }
            Err(mpsc::TryRecvError::Empty) => {
                executor.timer(POLL).await;
                waited += POLL;
            }
        }
    }
}

/// The files of the copy: the project laid out as its source export, or `None` while
/// the graphs have problems and make no code.
fn copy_files(state: &AppState) -> Option<BTreeMap<PathBuf, Vec<u8>>> {
    let output = rustbot_codegen::generate(&state.project, &state.registry).ok()?;
    Some(crate::export::source_files(&state.project, output))
}

/// Makes the copy follow the project, when code help runs.
pub(crate) fn sync(state: &AppState) {
    if let Some(server) = &state.analyzer.server
        && let Some(files) = copy_files(state)
    {
        server.sync(files);
    }
}

/// Starts code help the first time a file of the user's code opens, after asking to
/// install rust-analyzer when the private Rust has none (D-178).
pub(crate) fn ensure(state: &Entity<AppState>, window: &mut Window, cx: &mut App) {
    let app = state.read(cx);
    if app.analyzer.asked {
        return;
    }
    // shortcut: without the private Rust there is nothing to add rust-analyzer to; the
    // first Run installs it, and the next code tab opened asks.
    let Some(toolchain) = app
        .toolchain
        .clone()
        .filter(rustbot_toolchain::Toolchain::is_installed)
    else {
        return;
    };
    state.update(cx, |state, _| state.analyzer.asked = true);
    if toolchain.has_rust_analyzer() {
        start(state, cx);
        return;
    }
    let (install, later) = (t("help-install"), t("help-later"));
    let answer = window.prompt(
        PromptLevel::Info,
        &t("help-install-title"),
        Some(&t("help-install-detail")),
        &[install.as_str(), later.as_str()],
        cx,
    );
    let state = state.clone();
    window
        .spawn(cx, async move |cx| {
            if answer.await != Ok(0) {
                return;
            }
            let _ = cx.update(|_, cx| {
                state.update(cx, |state, cx| {
                    state.log(t("help-installing"));
                    cx.notify();
                });
            });
            let installed = cx
                .background_executor()
                .spawn(async move {
                    toolchain
                        .install_rust_analyzer(&mut |_| {})
                        .map_err(|e| e.to_string())
                })
                .await;
            let _ = cx.update(|_, cx| match installed {
                Ok(()) => start(&state, cx),
                Err(error) => state.update(cx, |state, cx| {
                    state.log_error(t_with("help-failed", &[("error", &error)]));
                    cx.notify();
                }),
            });
        })
        .detach();
}

fn start(state: &Entity<AppState>, cx: &mut App) {
    let app = state.read(cx);
    let Some(toolchain) = app.toolchain.clone() else {
        return;
    };
    let Some(files) = copy_files(app) else {
        // The next code tab opened tries again.
        state.update(cx, |state, cx| {
            state.analyzer.asked = false;
            state.log(t("help-waits"));
            cx.notify();
        });
        return;
    };
    let copy = toolchain.analyzer_dir(&app.project);
    let state = state.clone();
    cx.spawn(async move |cx| {
        let command = toolchain.rust_analyzer();
        let server = cx
            .background_executor()
            .spawn(async move { Server::start(command, &copy, files) })
            .await;
        state.update(cx, |state, cx| {
            match server {
                Ok(server) => {
                    let poll = cx.spawn(async move |this, cx: &mut gpui_kit::AsyncApp| {
                        loop {
                            cx.background_executor().timer(DIAGNOSTICS_POLL).await;
                            let alive = this.update(cx, |state, cx| {
                                let fresh = state.analyzer.server.as_ref();
                                if fresh.is_some_and(|server| server.take_fresh()) {
                                    cx.emit(CodeHelp);
                                }
                            });
                            if alive.is_err() {
                                break;
                            }
                        }
                    });
                    state.analyzer = Analyzer {
                        server: Some(server),
                        asked: true,
                        _poll: Some(poll),
                    };
                    cx.emit(CodeHelp);
                }
                Err(error) => state.log_error(t_with("help-failed", &[("error", &error)])),
            }
            cx.notify();
        });
    })
    .detach();
}

/// Code help for one file in the editor, through its provider hooks.
pub(crate) struct Help {
    server: Arc<Server>,
    path: PathBuf,
}

impl Help {
    pub(crate) fn new(server: Arc<Server>, path: PathBuf) -> Rc<Self> {
        Rc::new(Self { server, path })
    }

    /// The file and position of a request at `offset`, after sending the newest text.
    fn at(&self, text: &Rope, offset: usize) -> Value {
        self.server.change(&self.path, &text.to_string());
        json!({
            "textDocument": { "uri": self.server.uri(&self.path) },
            "position": text.offset_to_position(offset),
        })
    }

    fn ask<T: Send + 'static>(
        &self,
        method: &str,
        params: &Value,
        cx: &mut App,
        read: impl FnOnce(Value) -> anyhow::Result<T> + Send + 'static,
    ) -> Task<anyhow::Result<T>> {
        let answer = self.server.request(method, params);
        let executor = cx.background_executor().clone();
        cx.background_spawn(async move { read(wait(executor, answer).await?) })
    }
}

impl CompletionProvider for Help {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<CompletionResponse>> {
        let mut params = self.at(text, offset);
        params["context"] = serde_json::to_value(trigger).unwrap_or_default();
        self.ask("textDocument/completion", &params, cx, |answer| {
            let response: Option<CompletionResponse> = serde_json::from_value(answer)?;
            Ok(response.unwrap_or(CompletionResponse::Array(Vec::new())))
        })
    }

    fn is_completion_trigger(&self, _: usize, new_text: &str, _: &mut App) -> bool {
        new_text
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | ':'))
    }
}

impl HoverProvider for Help {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<Option<Hover>>> {
        let params = self.at(text, offset);
        self.ask("textDocument/hover", &params, cx, |answer| {
            Ok(serde_json::from_value(answer)?)
        })
    }
}

impl DefinitionProvider for Help {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<anyhow::Result<Vec<LocationLink>>> {
        let params = self.at(text, offset);
        self.ask("textDocument/definition", &params, cx, |answer| {
            let link = |location: Location| LocationLink {
                origin_selection_range: None,
                target_uri: location.uri,
                target_range: location.range,
                target_selection_range: location.range,
            };
            Ok(match serde_json::from_value(answer)? {
                None => Vec::new(),
                Some(GotoDefinitionResponse::Scalar(location)) => vec![link(location)],
                Some(GotoDefinitionResponse::Array(locations)) => {
                    locations.into_iter().map(link).collect()
                }
                Some(GotoDefinitionResponse::Link(links)) => links,
            })
        })
    }
}

/// Go to definition in another file of the project opens its tab there. `false` leaves a
/// place in `path` itself to the editor.
pub(crate) fn show_document(
    state: &Entity<AppState>,
    server: &Server,
    path: &Path,
    params: &lsp_types::ShowDocumentParams,
    cx: &mut App,
) -> bool {
    let Some(target) = server.project_path(params.uri.as_str()) else {
        // shortcut: a definition outside the project (djs-rs, std) is not shown; open the
        // library's source read-only if users ask for it.
        return true;
    };
    if target == path {
        return false;
    }
    if target.starts_with("src") || target.starts_with("generated") {
        let line = params.selection.map(|range| range.start.line);
        state.update(cx, |_, cx| cx.emit(OpenCode { path: target, line }));
    }
    true
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use std::path::{Path, PathBuf};

    use serde_json::json;

    #[test]
    fn messages_are_framed_with_their_length() {
        let mut out = Vec::new();
        let message = json!({ "jsonrpc": "2.0", "id": 1, "result": "zażółć" });
        super::write_message(&mut out, &message).unwrap();
        super::write_message(&mut out, &json!({ "method": "x" })).unwrap();
        let text = String::from_utf8(out.clone()).unwrap();
        // Bytes, not characters: "zażółć" is 10 bytes.
        assert!(text.starts_with("Content-Length: 46\r\n\r\n{"), "{text}");
        let mut input = BufReader::new(out.as_slice());
        assert_eq!(super::read_message(&mut input).unwrap(), Some(message));
        assert_eq!(
            super::read_message(&mut input).unwrap(),
            Some(json!({ "method": "x" }))
        );
        assert_eq!(super::read_message(&mut input).unwrap(), None);
        let mut too_long = BufReader::new(&b"Content-Length: 99999999999\r\n\r\n"[..]);
        assert!(super::read_message(&mut too_long).is_err());
    }

    #[test]
    fn uris_name_files_of_the_copy() {
        let copy = Path::new("/home/a b/.cache/analyzer/1");
        let uri = super::file_uri(&copy.join("src/lib.rs"));
        assert_eq!(uri, "file:///home/a%20b/.cache/analyzer/1/src/lib.rs");
        assert_eq!(
            super::in_copy(copy, &uri),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert_eq!(super::in_copy(copy, "file:///home/other/src/lib.rs"), None);
        let through = "file:///home/a%20b/.cache/analyzer/1/generated/src/../../src/lib.rs";
        assert_eq!(
            super::in_copy(copy, through),
            Some(PathBuf::from("src/lib.rs"))
        );
        let above = "file:///home/a%20b/.cache/analyzer/1/../2/src/lib.rs";
        assert_eq!(super::in_copy(copy, above), None);
        assert_eq!(
            super::file_path("file:///c%3A/Users/x/src/lib.rs"),
            Some(PathBuf::from("c:/Users/x/src/lib.rs"))
        );
        assert_eq!(
            super::in_copy(Path::new(r"C:\Users\x"), "file:///c%3A/Users/x/src/lib.rs"),
            Some(PathBuf::from("src/lib.rs"))
        );
        assert_eq!(
            super::file_uri(Path::new(r"C:\Users\x")),
            "file:///C:/Users/x"
        );
    }

    /// rust-analyzer on the copy finds the user's code through the `#[path]` module: it
    /// explains a function there on hover and reports a missing module (D-178).
    #[test]
    #[ignore = "needs the private Rust and the network; installs rust-analyzer into it"]
    fn a_real_rust_analyzer_helps_in_the_user_s_code() {
        use std::time::{Duration, Instant};

        let toolchain = rustbot_toolchain::Toolchain::locate().unwrap();
        assert!(toolchain.is_installed(), "install the private Rust first");
        toolchain
            .install_rust_analyzer(&mut |line| eprintln!("{line}"))
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("bot");
        let project = crate::workspace::new_project(&root).unwrap();
        let graph = json!({
            "schema": 1,
            "nodes": {
                "n1": { "type": "core.ready" },
                "n2": { "type": "core.custom_code", "props": {
                    "function": "on_start", "result": "text" } }
            },
            "edges": [ { "from": "n1.then", "to": "n2.run" } ]
        });
        project
            .save_graph(&rustbot_core::GraphFile {
                kind: rustbot_core::GraphKind::Event,
                name: "ready_code".to_owned(),
                graph: serde_json::from_value(graph).unwrap(),
                layout: rustbot_core::Layout::default(),
            })
            .unwrap();
        let code = "mod missing;\n\n/// Says hello.\npub async fn on_start(\n    ctx: &djs_rs::Context,\n) -> Result<String, djs_rs::HandlerError> {\n    Ok(String::new())\n}\n";
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), code).unwrap();
        let project = rustbot_core::Project::open(&root).unwrap();
        let registry = rustbot_core::registry::Registry::with_core();
        let output = rustbot_codegen::generate(&project, &registry).unwrap();
        let files = crate::export::source_files(&project, output);
        let copy = dir.path().join("copy");
        let server = super::Server::start(toolchain.rust_analyzer(), &copy, files).unwrap();
        let lib = Path::new("src/lib.rs");
        server.open(lib, code);

        let start = Instant::now();
        let limit = Duration::from_secs(600);
        let hover = loop {
            assert!(start.elapsed() < limit, "no hover in time");
            let params = json!({
                "textDocument": { "uri": server.uri(lib) },
                "position": { "line": 3, "character": 15 },
            });
            let answer = server
                .request("textDocument/hover", &params)
                .recv_timeout(Duration::from_secs(60))
                .unwrap();
            if let Ok(hover) = answer
                && !hover.is_null()
            {
                break hover;
            }
            std::thread::sleep(Duration::from_secs(1));
        };
        eprintln!("hover after {:?}: {hover}", start.elapsed());
        assert!(hover.to_string().contains("on_start"), "{hover}");
        assert!(hover.to_string().contains("Says hello"), "{hover}");
        while !server
            .diagnostics(lib)
            .iter()
            .any(|d| d.message.contains("missing"))
        {
            assert!(start.elapsed() < limit, "{:?}", server.diagnostics(lib));
            std::thread::sleep(Duration::from_millis(500));
        }
        eprintln!("diagnostics: {:?}", server.diagnostics(lib));
    }
}
