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

## Exact next step
Phase 2, step 3: the node registry in `rustbot-core`, as decided in D-052–D-054. It has
the slice's nodes `core.slash_command` (out pin `then`; props `name`, `description`;
provides `user`, `channel`, `server`) and `core.reply` (in pin `run`, out pin `then`;
prop `content` with templates). Validation reports unknown node types, unknown pins,
flow wired to data, missing required props, and unknown template variables. D-055 (test
server vs global) belongs to the runner and codegen steps.

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing yet. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) is being fixed (owner, 2026-11-17). It blocks only the later Cloudflare Workers
phase.
