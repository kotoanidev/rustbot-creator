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
builds the bot against djs-rs 0.1.0-alpha.1 with `clippy -D warnings` and runs as its own CI
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

## Exact next step
Phase 2, step 7: the runner in `rustbot-runner` (D-022, in-app part). It starts the built
bot (`Toolchain::build` returns its path) with `DISCORD_TOKEN` from `rustbot-secrets` and
`DISCORD_TEST_GUILD_ID` (D-056), passed only through the child's environment, never a
file. It streams stdout and stderr as log lines, and stops the bot (SIGTERM, then kill
after a timeout). The test server id is stored per user in the data folder, keyed by the
project id (D-065).

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing yet. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) is being fixed (owner, 2026-11-17). It blocks only the later Cloudflare Workers
phase.
