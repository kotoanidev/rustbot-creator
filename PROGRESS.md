# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Initialization: ✅ done 2026-11-17.**
**Phase 1 — Detailed planning: ✅ done 2026-11-17** for everything Phase 2 needs (D-011 to
D-046). Later phases are planned in detail when each one starts (D-043).
**Phase 2 — Vertical slice: in progress.** Step 1 (workspace skeleton + CI) is ✅ done.
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

Step 9 (budgets) is in progress; see `docs/PERFORMANCE.md`. Startup (warm ~0.2 s) and
memory (108–130 MB) are within budget. The 500-node canvas now draws only what is on
screen, with one path per wire colour and no shadows: 60 FPS at the median, but p99 is
26–33 ms against 16.6 ms, because of rare CPU spikes whose cause is open. A cold first
start takes 1.09 s.

## Exact next step
1. **Live test with the owner** (the owner does it, because only they may enter the
   token): open or create a project, click empty canvas, paste the bot token and the test
   server id under Properties, press Run, accept the questions (trust, Rust install), use
   `/ping` on the test server and expect "Pong, <name>!", then Stop.
2. Find the frame spikes on the 500-node graph (p99). The `RUSTBOT_MEASURE=frames` mode
   measures them. Then mark Phase 2 done in `ROADMAP.md`.

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) was fixed in djs-rs 0.1.0-alpha.2 (2026-11-22), which new projects use (D-069), so
the Cloudflare Workers phase is no longer blocked.
