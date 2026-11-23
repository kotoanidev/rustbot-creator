# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Initialization: ✅ done 2026-11-17.**
**Phase 1 — Detailed planning: ✅ done 2026-11-17** for everything Phase 2 needs (D-011 to
D-046). Later phases are planned in detail when each one starts (D-043).
**Phase 2 — Vertical slice: ✅ done 2026-11-23.** The owner ran `/ping` live on a test server.
**Phase 3 — Easy mode: in progress.** Planned 2026-11-23 (D-073–D-075). Step 1, the console (D-075, C-014), is ✅ done: lines select and copy with Ctrl+C, errors are red and warnings yellow, build output is muted, and "Clear" sits in the panel's toolbar. Step 1 (workspace skeleton + CI) is ✅ done.
The six crates of D-036 and the lints of D-046 are in place, and `cargo run -p
rustbot-creator` opens an empty "RustBot Creator" window (gpui-kit 0.10.0). The first CI
run was green: rustfmt 15 s, Ubuntu 280 s, Fedora 44 297 s.
Step 2 (project format v0, D-047–D-051, C-003) is ✅ done. `rustbot-core` has `Manifest`,
`Graph`, `Layout`, `GraphFile` and `Project::{create, open, save_manifest, save_graph}`,
with 12 tests: the round trip, byte-exact deterministic files, refusing to overwrite, the
schema check, unknown fields, graph names that would escape the folder, and symlinks
(refused on read and write, and a planted temporary file is never followed, from the
security review fix `f92e0e2`).

Step 3 (node registry, D-052–D-054) is ✅ done. `rustbot_core::registry` has `NodeType`,
`Pin`, `Prop`, `Variable` and `DataType`, all serde data for the future WASM plugins, plus
`Registry::{with_core, register, get, iter, validate}` and `Problem`/`ProblemKind` for the
"Problems" panel. `rustbot_core::template` parses `{path}` and `{{`/`}}`. The core nodes
are `core.slash_command` (out `then`; props `name` and `description`; provides `user`,
`channel` and `server`) and `core.reply` (in `run`, out `then`; prop `content` with
templates). 24 tests pass in `rustbot-core`.

Step 4 (code generation, D-056–D-059, C-004) is ✅ done. `rustbot_codegen::{generate,
write}` turn a validated project into `generated/` (`Cargo.toml`, `src/main.rs`,
`src/settings.rs`, `src/commands/{mod,<name>}.rs`), using tokens and prettyplease, with
snapshot tests (`insta`), injection tests, and the `generated_bot_compiles` test. That test
builds the bot against djs-rs 0.1.0-alpha.2 (D-069) with `clippy -D warnings` and runs as its own CI
job. 501 nodes generate in 7.6 ms (`docs/PERFORMANCE.md`). `rustbot-core` gained
`Project::{write_file, remove_file, files_under}`, which are symlink-safe.

Security review fix `fix(codegen)`: `write` owns all of `generated/` (it removes planted
`build.rs` and `.cargo/`), the bot's `Cargo.toml` sets `build = false` and turns off
`auto*`, and `generated/rust-toolchain.toml` pins Rust (D-062).

Step 5 (private toolchain, D-061–D-064, C-005) is ✅ done. `rustbot_toolchain::Toolchain`
has `locate`, `install`, `is_installed`, `build` and `target_dir`, plus `c_compiler()` and
`install_c_compiler_command()`. The real end-to-end test (`--ignored install`) installs
in 29 s, builds `/ping` in 42 s, and rebuilds in 1.1 s (`docs/PERFORMANCE.md`). Project
trust (D-064) is decided and belongs to the app shell step.

Step 6 (bot token in the keyring, C-006) is ✅ done. `rustbot_secrets::{bot_token,
set_bot_token, delete_bot_token}` are keyed by the project id, and the round trip passes
against KDE Wallet. Fix `4c7b764`: cargo runs in the cache folder with `--manifest-path`,
so `.cargo/config.toml` in or above a project is never read (security review).

Step 7 (runner, D-065, C-007) is ✅ done. `rustbot_runner::Bot::{start, exit_status,
stop}` runs a built bot with the token and test server in its environment, streams its
output as `LogLine`s, and stops it with SIGINT, then a kill after a grace period.
`rustbot_runner::{test_server, set_test_server}` keep the test server per user.
`Toolchain::bot_dir` is the project's folder in the app's data. The runner has not been
tried on a real built bot with a real token yet: that is the live test of step 9.

Step 8 (app shell, D-066–D-068, C-008, C-009) is in progress.
- 8a ✅ The window has a welcome screen (New/Open project through the system dialogs) and
  the dock: the project tree and Properties on the left, the editor in the centre, Logs
  and Problems below, and a status bar. All text is in Fluent, English and Polish.
- 8b ✅ The graph canvas (`src/panels/editor.rs`, `src/canvas.rs`) draws nodes, pins and
  wires. You can drag nodes, select one, wire pins by dragging, delete with Delete or
  Backspace, add nodes from a searchable list (right-click, "+", or a wire dropped on
  empty canvas, D-067), pan, and undo and redo with Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y.
  Changes are autosaved (D-068). Seen on screen; dragging and autosave were tried in the
  real window. Headless UI tests (D-070) drag a node, drop a wire to add a wired node,
  delete and then undo and redo, and search the right-click list. They found that a
  right-click took the focus back from the list's search, so the list now opens on
  release.

- 8c ✅ Properties (C-011): the selected node's fields, with translated labels and the
  template variables, edited with undo. With nothing selected, the bot settings: the
  masked token saved to the keyring and the test server. A headless test checks that
  typing edits the node and that undo restores the field. The form was seen on screen.

- 8d ✅ Run/Stop in the status bar (D-071, C-012), in `src/run.rs` and `src/trust.rs`:
  save → problems → trust (D-064) → gcc and Rust with consent → codegen → build →
  token and test server → start, with every line in Logs; Stop stops the bot. A
  headless test clicks Run on a project from elsewhere and checks that the trust
  question comes first and that Cancel builds nothing.

Step 8 (app shell) is ✅ done.

Step 9 (budgets) is measured, and every budget is met (`docs/PERFORMANCE.md`): warm
startup 150–280 ms, memory 108–130 MB, frame time with 500 nodes p99 6.8–10 ms, codegen
7.6 ms, rebuild 1.1 s. The canvas draws only what is on screen, with one path per wire
colour, and skips drawing in the first frame. Only a cold first start after a build is
slower (1.09 s).

The owner's first live run (2026-11-23) worked up to Run, then failed with 403. Discord
refuses to register commands in a server the bot is not on (`50001 Missing Access`).
The fixes:
- The generated bot checks the Ready server list (D-072). When the test server is
  missing, it prints a plain message and an invite link on its own line. The Logs panel
  opens such a line in the browser.
- The runner sets `NO_COLOR=1`, so tracing's colour codes no longer clutter the Logs.

After these fixes the owner's live test passed (2026-11-23): Run built the bot, and
`/ping` answered on the test server. Step 9 and Phase 2 are ✅ done.

The owner finds the console hard to read and wants it improved later. Colour codes are
gone, but selecting and copying text, levels and wrapping are still open.

## Exact next step
**Phase 3, step 2b: the variable picker (D-078).** Step 2a, command options, is done:
- registry and codegen (C-015);
- the editor in Properties, tested headlessly;
- replies that notify only what their author wrote (C-016, from a security review).

2b covers two things. Fields of type user, channel or role become drop-downs of the
variables that fit; `Registry::scope` already lists them with their types. Text fields get
an {x} button that inserts a variable at the cursor. The nodes that need typed fields
(moderation, D-073) come in 2c–2d, so 2b may build the typed field together with the first
such node.

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) was fixed in djs-rs 0.1.0-alpha.2 (2026-11-22), which new projects use (D-069), so
the Cloudflare Workers phase is no longer blocked.
