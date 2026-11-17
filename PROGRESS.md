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
djs-rs#3 filed for HTTP interactions + wasm32 (D-018), a folder of TOML + JSON as the
project format (D-019), OS keyring with an encrypted-file fallback for secrets (D-020),
GitHub OAuth device flow (D-021), and an in-app process plus a background service for
self-hosting (D-022), MIT-0 generated code (D-023), EN + PL UI with i18n (D-024), RPM on
Copr first (D-025), a gate plus performance budgets as the quality bar (D-026), the latest
stable Rust with no MSRV (D-027), a multi-crate workspace (D-028), the `rustbot-*` crate
prefix (D-029), rust-analyzer as a rustup component (D-030), a vertical slice first (D-031),
the easy mode widened first (D-032), the scope of the first release (D-033), and themes in
the first release with plugins after it (D-034).

## Exact next step
1. Propose to the owner the order of phases inside the first release (D-033), for
   approval. Then write it into `ROADMAP.md`.
2. Settle the open questions the vertical slice needs: the exact crate list, the project
   layout and schema (D-019), the toolchain details (compiler/linker on Fedora,
   `zstd-stream` vs `zlib-stream`), the i18n library, and the performance budget values
   (D-026).
3. The remaining open questions (plugin details, VS Code themes, other hosts, OAuth App vs
   GitHub App, community files) wait until the phase that needs them.

## Blocked on the owner
- Every open question in `docs/DECISIONS.md` (D-006: the owner decides).

## Blocked on djs-rs
Nothing yet. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) is being fixed (owner, 2026-11-17). It blocks only the later Cloudflare Workers
phase.
