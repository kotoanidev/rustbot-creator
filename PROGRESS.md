# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Initialization: ✅ done 2026-11-17.** The repository holds the decision log,
licenses, README, CLAUDE.md, roadmap and this file. There is no code yet.

**Phase 1 — Detailed planning: in progress.** Decided so far: gpui + gpui-kit for the UI
(D-011), bots as generated Rust code (D-012), Cloudflare Containers first and Workers later
(D-013), forms + node graph in the easy creator (D-014), one project with two views (D-015),
WASM plugins in wasmtime (D-016), a private toolchain managed by the app (D-017), and
djs-rs#3 filed for HTTP interactions + wasm32 (D-018).

## Exact next step
Continue through the remaining "Open questions" in `docs/DECISIONS.md`, next: the project
format on disk, secrets and token storage, GitHub integration, and self-hosting on the
user's PC. Ask, record each answer as `D-NNN`, then write the phased plan into
`ROADMAP.md`.

## Blocked on the owner
Every open question in `docs/DECISIONS.md` (D-006: the owner decides).

## Blocked on djs-rs
Nothing yet. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) will block the later Cloudflare Workers phase until it lands.
