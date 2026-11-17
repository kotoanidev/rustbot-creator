# Decision log

Every significant decision in RustBot Creator is recorded here. Decisions marked **Owner**
were made by the project owner and are binding. Changing one requires asking the owner again.

Format: `D-NNN`, then a title. Each entry gives the date, who decided, the decision, the
alternatives that were considered, and why.

---

## Owner decisions

### D-001 — Name: RustBot Creator, repository `rustbot-creator`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The application is called **RustBot Creator**. The repository is
  `github.com/kotoanidev/rustbot-creator`. Crate names are still open.
- **Alternatives:** repository `RustBot-Creator`, `rustbot`.

### D-002 — Public repository, work directly on `main`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The repository is public from the start. Every significant change is committed
  to `main` and pushed immediately, so work survives interrupted sessions. No feature branches.
- **Why:** unlimited free GitHub Actions minutes, including the Windows and macOS runners,
  and the community can follow the project from day one.
- **Alternatives:** private until the first release; feature branches + PRs.

### D-003 — License of the application: `MIT OR Apache-2.0`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The application is dual-licensed under MIT or Apache-2.0, at the user's option.
  This is the same license as djs-rs. The license of the code the app *generates* for users'
  bots is a separate, open question.
- **Why:** it is the Rust ecosystem convention, and a permissive license lets community
  plugins choose their own license.
- **Alternatives:** GPL-3.0-or-later, MPL-2.0.

### D-004 — Working process copied from djs-rs
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The project keeps this decision log, a resumable `PROGRESS.md` and a phased
  `ROADMAP.md`, and uses [Conventional Commits](https://www.conventionalcommits.org/).
- **Alternatives:** the same process with branches + PRs; the same process with Polish
  repository content.

### D-005 — Language
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Claude talks to the owner in Polish. Everything in the repository is in
  English: code, identifiers, docs and commit messages. The languages of the app's UI are
  still open.

### D-006 — The owner makes every significant decision
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Claude never makes a significant decision on the owner's behalf. It asks,
  offering options and marking a recommendation, then records the answer here. This covers
  architecture, dependencies, UI framework, file formats, public interfaces, naming, scope,
  phase order, licensing, publishing and repository settings.

### D-007 — Pure-Rust desktop application, bots built on djs-rs
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The application is written in pure Rust. The bots it creates use
  [djs-rs](https://github.com/kotoanidev/djs-rs), which was at `0.1.0-alpha.1` when this was
  decided.

### D-008 — djs-rs problems are reported, not fixed here
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** djs-rs is in alpha. Its bugs, gaps and remarks about it are filed as issues
  in `kotoanidev/djs-rs`, where another agent handles them. This project never patches or
  works around djs-rs internally. If a djs-rs problem blocks progress, work stops and waits
  for a new djs-rs release.

### D-009 — Platform order: Linux (Fedora first), then Windows, macOS later
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Fedora is the priority platform and the test environment. Linux support then
  extends to all major distributions. Windows comes after Linux is finished. macOS is only a
  future prospect, but nothing should rule it out.

### D-010 — Product scope (owner's brief)
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The brief that planning will turn into a detailed plan:
  - **Easy creator:** someone with no programming knowledge can build a complete bot. It
    should be like a "start it"-style panel but much freer, with no limits, able to do
    absolutely anything.
  - **Advanced creator:** control over every feature. Building a bot needs some skill but
    stays simple and convenient. It includes a built-in file editor.
  - **UI:** similar to VS Code, professional, no neon, with a foundation for community
    plugins and themes.
  - **Dependencies:** the application installs all of its dependencies by itself.
  - **Priorities:** a pleasant, easy experience, and performance.
  - **Integrations:** GitHub built in, plus hosts (Cloudflare and other compatible ones).
    Integrations with more apps and services will be chosen later.
  - **Output:** a bot can be downloaded as source code or as a ready deploy, hosted from the
    app on the user's own PC, or deployed to Cloudflare or other popular compatible hosts.

### D-011 — UI framework: gpui + gpui-kit
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The UI is built on [gpui](https://crates.io/crates/gpui), the GPU-accelerated
  framework behind the Zed editor, through [gpui-kit](https://github.com/longbridge/gpui-kit).
  gpui-kit is Apache-2.0 and was at v0.10.0 on 2026-11-16. It pins the matching gpui
  release and provides a code editor (Tree-sitter, LSP diagnostics, completion and hover), a
  serializable dock layout with draggable tabs, JSON themes, 75+ components, AccessKit
  accessibility and headless UI tests.
- **Risk accepted:** both are pre-1.0 and their APIs change often.
- **Alternatives:** iced 0.14 (no ready dock or code editor), egui 0.37 (harder to make look
  professional, weaker editor), Slint 1.19 (GPLv3 / royalty-free / commercial license).

### D-012 — Bots are generated Rust code
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Both creator modes produce a real Rust project on djs-rs, compiled by cargo.
  There is one source of truth, no interpreter, full native performance, and "source code"
  is the natural output. The PC therefore needs a Rust toolchain and a linker or C compiler,
  which the app installs (D-010). The first build takes minutes, and later rebuilds take
  seconds.
- **Alternatives:** a prebuilt engine interpreting a bot description; a hybrid with an
  interpreter for preview and code generation for export.

### D-013 — Cloudflare: Containers first, Workers later
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Full gateway bots are deployed to Cloudflare as Linux containers
  (Cloudflare Containers, kept alive), which needs no changes in djs-rs. Workers, which
  can run only HTTP-interactions bots, come in a later phase. They depend on djs-rs
  supporting `wasm32` and an HTTP interactions server, neither of which exists in
  `0.1.0-alpha.1`.
- **To verify when the phase is planned:** Containers pricing and plan (likely Workers
  Paid), keep-alive configuration, and outbound WebSocket behaviour.
- **Alternatives:** Workers only; both from the start; Cloudflare after other hosts.

### D-014 — Easy creator: forms + node graph
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** In the easy creator, simple settings (command name, description,
  permissions, …) are edited in forms, and logic is a node graph wired by dragging, e.g.
  `/ban → has permission? → yes: ban user → reply` / `no: reply`.
- **Alternatives:** node graph only; Scratch-like blocks; forms + a list of steps.

---

## Defaults chosen by Claude (reviewable)

### C-001 — Repository housekeeping
- **Date:** 2026-11-17
- **Choice:** `.editorconfig` and the license texts are taken from djs-rs, with the copyright
  holder "The RustBot Creator contributors". `.gitignore` covers the Rust build output,
  editor files and `.env` secrets. There is no Cargo workspace, toolchain file or CI yet,
  because their shape depends on open questions below.

---

## Open questions (to ask the owner when relevant)

The agenda for detailed planning. Each one becomes a `D-NNN` once the owner answers it.

- **Workspace layout and crate names.**
- **Toolchain:** edition, MSRV policy, `rust-toolchain.toml`.
- **How the two modes relate** (D-012, D-014): one project with two views, a one-way
  "eject", or full two-way sync between the graph and the code.
- **The advanced creator and the editor:** gpui-kit's editor (D-011) plus rust-analyzer as
  the LSP server, and how it is installed.
- **Project format:** how a bot project is stored on disk and versioned.
- **Plugins:** mechanism (WASM, dynamic libraries, scripting), sandboxing and permissions,
  plugin API stability, distribution (a registry?).
- **Themes:** format, and compatibility with VS Code themes.
- **Self-installing dependencies:** rustup/toolchain management, a C compiler (djs-rs's
  default `zstd-stream` needs one), CMake (the `voice` feature), and where they are installed.
- **Other hosts** besides Cloudflare (D-013), and when to ask djs-rs for `wasm32` + an
  HTTP interactions server for the Workers phase.
- **Self-hosting on the user's PC:** running in the background, autostart, logs, restarts.
- **GitHub integration:** authentication (OAuth device flow?), token storage (OS keyring),
  scope of features.
- **Secrets:** bot tokens, and where and how they are stored.
- **Linux packaging and updates:** Flatpak, RPM, AppImage, .deb, Copr, auto-update.
- **App UI languages / i18n.**
- **License of generated bot code and templates.**
- **Quality bar:** CI, tests, benchmarks (djs-rs requires them for every stage, D-041
  there), lints.
- **Community files:** CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, issue templates, and when
  to add them.
