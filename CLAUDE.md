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

**State: planning. There is no code, Cargo workspace or CI yet.** The architecture, commands
and layout get documented here once the planning answers settle them.

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
Talk to the owner in **Polish**, with correct diacritics. Everything in the repository is in
**English**.

## Platform priority (D-009)

Fedora (the owner's machine, Fedora 44 KDE) is the priority platform and the test
environment. Then come all major Linux distributions, then Windows. macOS is only a
prospect, but never rule it out.

## djs-rs quick reference

- crates.io `djs-rs`, guide https://kotoanidev.github.io/djs-rs/, API https://docs.rs/djs-rs
- Edition 2024, MSRV 1.89, tokio. Its default feature `zstd-stream` needs a C compiler, and
  `voice` needs CMake. Both matter for "the app installs its own dependencies".
