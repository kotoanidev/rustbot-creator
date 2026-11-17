# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

**RustBot Creator** is a pure-Rust desktop application for building Discord bots on top of
[djs-rs](https://github.com/kotoanidev/djs-rs), the owner's own discord.js-like Rust library
(alpha). It has an easy no-code creator and an advanced creator with a built-in editor, a
VS Code-like UI with plugins and themes, GitHub and hosting integrations, and several ways to
ship a bot. The full brief is D-010 in [`docs/DECISIONS.md`](docs/DECISIONS.md).

- Repository: https://github.com/kotoanidev/rustbot-creator (public)
- License: `MIT OR Apache-2.0`
- Local directory: `discord-bot-rust-app` (not renamed to match the repository)

**State: Phase 2, the vertical slice (see `ROADMAP.md`).** The workspace skeleton and CI
exist. The app opens an empty window.

## Commands

```bash
pkexec scripts/install-build-deps.sh                 # once: the system libraries gpui needs (dnf/apt)
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p rustbot-core <test_name>               # a single test
cargo run -p rustbot-creator                         # the app
```

The local quality gate before every code commit is `fmt` → `clippy` → `test`. CI
(`.github/workflows/ci.yml`) runs it on a `fedora:44` container and on `ubuntu-latest`
(D-045).

## Architecture (decided; mostly still to be built)

The pipeline below is the vertical slice (D-031). The crates exist from D-036, and later
phases add `rustbot-github`, `rustbot-cloudflare`, `rustbot-plugin-host` and
`rustbot-plugin-sdk`.

```
bot project folder (D-019, D-040)
  project.toml · graphs/{commands,events}/*.json · src/ (user's Rust) · generated/ (committed)
        │
rustbot-core       loads/saves the project, graph model, node registry
        │
rustbot-codegen    graph → Rust on djs-rs, written to generated/ (deterministic)
        │
rustbot-toolchain  private rustup + Rust in the app's XDG data dir (D-017); system gcc
        │          installed only with consent (D-039); runs cargo build
rustbot-runner     runs the bot as a child process, streams logs; later a systemd --user
        │          service (D-022)
rustbot-creator    the gpui-kit app: VS Code-like dock (D-044), node-graph canvas (D-042)
rustbot-secrets    bot/GitHub/Cloudflare tokens in the OS keyring (D-020)
```

Invariants that span crates:
- The graph and forms are the source of truth. `generated/` is never edited by hand, and
  the user's own code lives in `src/` and is called through a "custom code" node (D-015).
- Built-in nodes and panels are registered the same way plugins will register theirs
  (D-034).
- All UI text goes through Fluent, in English and Polish (D-024, D-037). gpui-kit's own
  strings use rust-i18n.
- gpui-kit pins its own gpui (`gpui-pre`). Any gpui-based crate must use that exact
  version, or it cannot share the window (D-042).
- Library crates use `thiserror`, and the app uses `anyhow` (D-041). No dependency outside
  D-041 is added without asking.

## Start-of-session checklist (mandatory)

1. Read [`PROGRESS.md`](PROGRESS.md) for the current state and the **exact next step**.
2. Read [`docs/DECISIONS.md`](docs/DECISIONS.md). Every decision in it is binding.
3. Skim [`ROADMAP.md`](ROADMAP.md) to see where the next step fits.
4. Run `git status` and `git log --oneline -10`. Uncommitted work comes from an interrupted
   session: inspect it, then finish or discard it **after asking the owner**.

## Hard rules

### 1. Never make significant decisions for the owner (D-006)
Architecture, dependencies (adding **any** dependency counts), UI framework, file formats,
public interfaces, naming, scope, phase order, licensing, publishing and repository
settings all need the owner's answer.
- Stop and ask in Polish, offer options, and mark your recommendation.
- Record the answer as the next free `D-NNN` in `docs/DECISIONS.md` **before** you
  implement it.
- You may make minor, mechanical choices directly. Log any that are not obvious under
  "Defaults chosen by Claude" (`C-NNN`).
- Questions that must be answered later go under "Open questions".

### 2. Commit and push after every significant change (D-002, D-004)
- Work directly on `main`, with no feature branches.
- After each meaningful unit of work, update `PROGRESS.md`, then `git commit` **and**
  `git push`.
- Write commit messages as Conventional Commits, e.g. `feat(editor): …` or `docs: …`.
- Never push code that fails the quality gate. Once a gate exists, it will be listed here.

### 3. djs-rs is not fixed here (D-008)
djs-rs is alpha. If you find a bug, a gap or an awkward API in it, **file an issue** in
`kotoanidev/djs-rs`, where another agent handles it. Do not patch it, vendor it or hack
around it:
```bash
gh issue create -R kotoanidev/djs-rs --title "…" --body "…" --label bug   # or enhancement
```
The repository has labels such as `crate: rs`, `crate: rest`, `crate: gateway` and
`crate: voice`. If a djs-rs problem blocks progress, **stop**, record it under "Blocked on
djs-rs" in `PROGRESS.md`, and wait for a new djs-rs release.

### 4. Keep `PROGRESS.md` resumable
It must always answer four questions: what is done, what is in progress, what exactly comes
next, and what is blocked on the owner or on djs-rs. A fresh session must be able to
continue from it alone.

### 5. Language (D-005)
Talk to the owner in **Polish**, with correct diacritics. The repository is **100% English**:
code, comments, docs, commit messages, issues, PRs, and anything you file in other
repositories (e.g. djs-rs). The **only exception** is the Polish UI translation files
(D-024).

## Platform priority (D-009)

Fedora (the owner's machine, Fedora 44 KDE) is the priority platform and the test
environment. Then come all major Linux distributions, then Windows. macOS is only a
prospect, but never rule it out.

## djs-rs quick reference

- crates.io `djs-rs`, guide https://kotoanidev.github.io/djs-rs/, API https://docs.rs/djs-rs
- Edition 2024, MSRV 1.89, tokio. Its default feature `zstd-stream` needs a C compiler, and
  `voice` needs CMake. Both matter for "the app installs its own dependencies".
