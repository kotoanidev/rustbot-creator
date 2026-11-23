# Roadmap

The phased plan for RustBot Creator. Checkboxes are updated as work lands on `main`. Scope
decisions are in [`docs/DECISIONS.md`](docs/DECISIONS.md). The phase order is D-035.

Legend: `[x]` done · `[~]` in progress · `[ ]` not started

A phase is done only when it passes the quality gate and its performance budgets are
measured (D-026, D-038).

---

## Phase 0 — Initialization ✅
- [x] Decision log, CLAUDE.md, roadmap, progress file
- [x] Licenses (MIT OR Apache-2.0), README
- [x] Public GitHub repository

## Phase 1 — Detailed planning
- [x] Answer the open questions in `docs/DECISIONS.md` that the next phase needs
- [x] Phase order of the first release (D-035)

---

# Release 1 — Fedora (D-033)

## Phase 2 — Vertical slice (D-031) ✅
Goal: a `/ping` command made in the graph runs as a real bot, end to end through every
layer, before the scope widens.
- [x] Cargo workspace with the six slice crates (D-036), lints, rustfmt/clippy config, CI on
      Linux (fmt, clippy, tests); first CI run green on Fedora 44 and Ubuntu
- [x] Project format v0: `project.toml` + graph JSON + layout JSON (D-019, D-051), in
      `rustbot-core` with tests
- [x] Node registry in `rustbot-core`, built the way plugins will use it (D-034, D-052),
      with the slice's nodes `core.slash_command` and `core.reply`, graph validation and
      templates (D-047, D-053, D-054)
- [x] Code generation of the `/ping` project on djs-rs (D-012, D-015, D-056–D-059); the
      generated bot builds against djs-rs 0.1.0-alpha.2 (CI job, D-069), 501 nodes in 7.6 ms
- [x] Private toolchain: install rustup + Rust into the app's data directory and build the
      bot (D-017, D-039, D-061–D-063); install 29 s, first build 42 s, rebuild 1.1 s
- [x] Bot token in the OS keyring (D-020, D-050), tested against KDE Wallet
- [x] Runner: start and stop the bot, with live logs in the app (D-022, in-app part);
      the log panel itself comes with the app shell
- [x] App shell: window, dock, project tree, minimal graph canvas, form panel, log panel,
      English + Polish text (D-024, D-037)
- [x] Budgets measured (D-038), all met; live test with the owner's test bot passed on
      2026-11-23 (`/ping` answered on the test server)

## Phase 3 — Easy mode (D-014, D-032) — in progress
- [x] Console step (D-075): select and copy, level colours, Clear, build lines apart from
      the bot's
- [x] The `/ban` example of D-014 (D-073), with values from form fields (D-074):
  - [x] Command options as a list in the command's form (D-076)
  - [x] Variable picker: lists for user/channel/role fields, {x} for text (D-078)
  - [x] Conditions with "yes"/"no" outputs: has permission, has role, compare (D-077)
  - [x] Moderation: ban, kick, timeout, add/remove a role
  - [x] Messages: send to a channel, a reply only the author sees, a basic embed (D-079)
- [ ] Full node catalog and forms over what djs-rs covers: events, components, and so on
  - [x] Events: member joined/left, new message, bot ready, reactions (D-091–D-095)
  - [x] Adding commands and events from the project tree, deleting graphs (D-092)
  - [x] Buttons and select menus: clicks that always work or are waited for (D-096–D-100)
  - [ ] More actions
- [x] Validation and errors shown on the graph
- [ ] Bot templates

## Phase 4 — Advanced mode (D-015, D-030)
- [ ] gpui-kit code editor for the user's own modules in `src/`
- [ ] rust-analyzer (LSP) from the private toolchain
- [ ] "Custom code" node calling the user's functions
- [ ] Read-only view of the generated code

## Phase 5 — Themes + first run (D-034, D-017)
- [ ] JSON themes, built-in dark and light, theme switching
- [ ] First-run wizard: toolchain installation, system packages with consent
- [ ] Settings

## Phase 6 — Export
- [ ] Source code (MIT-0 templates, the user's chosen license, D-023)
- [ ] Ready build

## Phase 7 — Hosting on the PC (D-022)
- [ ] `systemd --user` service with autostart and restarts; status and logs in the app

## Phase 8 — GitHub (D-021)
- [ ] Device flow sign-in (OAuth App `Ov23liQbIFt9vqboIA0x`), create a repository, push
      the project

## Phase 9 — Cloudflare Containers (D-013)
- [ ] One-click deploy of the export's build as a kept-alive container

## Phase 10 — Release 1 (D-025)
- [ ] RPM on Copr, testing on Fedora, release

---

# After release 1 (order still to be decided)
- WASM plugins (D-016, D-034)
- Other Linux distributions and packages: .deb, AUR, openSUSE, tarball (D-009, D-025)
- Windows (D-009)
- Cloudflare Workers (D-013), on djs-rs 0.1.0-alpha.2's `http-interactions` and wasm32
  support ([djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3), fixed)
- macOS is a prospect only (D-009)
