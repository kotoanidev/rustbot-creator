# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Initialization: ✅ done 2026-11-17.**
**Phase 1 — Detailed planning: ✅ done 2026-11-17** for everything Phase 2 needs (D-011 to
D-046). Later phases are planned in detail when each one starts (D-043).
**Phase 2 — Vertical slice: in progress.** Step 1 (workspace skeleton + CI) is done
locally. The six crates of D-036 and the lints of D-046 are in place, the gate passes on
Fedora 44, and `cargo run -p rustbot-creator` opens an empty "RustBot Creator" window
(gpui-kit 0.10.0). The first CI run on GitHub is pending.

## Exact next step
1. Check that the first CI run is green, especially the Ubuntu package list in
   `scripts/install-build-deps.sh`, which has not been tested yet. Fix it if needed, then
   tick step 1 in `ROADMAP.md`.
2. Phase 2, step 2: project format v0 in `rustbot-core` (`project.toml` + one graph JSON,
   D-019/D-040), with tests.

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing yet. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) is being fixed (owner, 2026-11-17). It blocks only the later Cloudflare Workers
phase.
