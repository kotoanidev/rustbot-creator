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
- **Decision:** Claude talks to the owner in Polish. The repository is **100% English**:
  code, identifiers, comments, docs, commit messages, issue and PR texts, and the
  repository description. The **only exception** is the Polish UI translation files (e.g.
  `locales/pl/`, D-024). This was confirmed by the owner on 2026-11-17. Text Claude files
  in other repositories, such as djs-rs issues, is in English too.

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
- **Amended by D-124:** Windows comes right after the public Linux release.

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

### D-015 — One project, two views
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** A bot is one project. Its graph and forms are the source of truth. The Rust
  code generated from them lives in a separate directory and is never edited by hand. In
  the advanced mode the user writes their own Rust modules, which the graph calls through a
  "custom code" node. The user can switch between the modes in both directions without
  losing work.
- **Alternatives:** one-way "eject" to a plain Rust project; full two-way graph↔code sync
  (arbitrary Rust cannot be shown faithfully as a graph); separate project types.

### D-016 — Plugins run as WASM in wasmtime
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Community plugins (logic, panels, new nodes, integrations) are WebAssembly
  modules run in wasmtime, as Zed's extensions are. They are sandboxed and get each
  capability as a separate grant. They can be written in Rust or any language that
  compiles to WASM, and the engine is pure Rust. Plugins do not draw UI themselves; they
  describe it through the app's API. Themes are data files (JSON, gpui-kit's format). That
  was stated in the question and not objected to, and VS Code theme import is still open.
- **Alternatives:** JS through gpui-kit's `gpui-shell` (QuickJS, written in C);
  native `.so`/`.dll` (no sandbox, no stable Rust ABI); declarative-only first.

### D-017 — The app manages a private toolchain
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The app installs its own rustup and Rust toolchain into its data directory,
  at the version the generated code needs. It never touches the user's `~/.cargo` or
  `~/.rustup`. System packages, such as a C compiler or linker, are installed only with the
  user's consent, through the system's administrator prompt. This costs about 1 GB of disk.
- **Alternatives:** reuse the user's rustup and install the missing parts; ask on first
  run; build inside Podman/Docker containers.

### D-018 — djs-rs asked for HTTP interactions + wasm32 now
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The need for the Workers phase (D-013) was filed right away as
  [kotoanidev/djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3), so djs-rs has time
  before that phase.
- **Alternatives:** file it when the Workers phase is planned.

### D-019 — Project format: a folder of TOML + JSON files
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** A bot project is a folder of text files, readable in git diffs and
  mergeable. Settings are in `project.toml`. Graphs are JSON, one file per command or
  event. The user's own Rust modules are in `src/`. Generated code (D-015) is in a separate
  directory. The exact layout and schema versioning are still open.
- **Alternatives:** a folder of RON; a folder of JSON only; a single SQLite file.

### D-020 — Secrets: OS keyring, encrypted file as fallback
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Bot tokens and GitHub and Cloudflare tokens are stored in the OS keyring
  (Secret Service via KDE Wallet or GNOME Keyring, Windows Credential Manager, macOS
  Keychain). Where no keyring exists, they go to an encrypted file unlocked by a master
  password. Secrets never enter project files or git.
- **Alternatives:** keyring only; encrypted file only.

### D-021 — GitHub sign-in: OAuth device flow
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The user clicks "Connect" and types a short code on github.com. The app holds
  only a public client ID, with no client secret. On 2026-11-17 the owner registered a
  **GitHub OAuth App** and handed its management to Claude. Its client ID is
  `Ov23liQbIFt9vqboIA0x`. That value is public, but the client secret must never enter the
  repository. "Enable Device Flow" is on in the app's settings: `POST
  https://github.com/login/device/code` with this client ID returned a device code on
  2026-11-17.
- **Alternatives:** a pasted Personal Access Token; OAuth with a PAT as an advanced option.

### D-022 — Self-hosting: in-app process + background service
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** While the user works, a bot runs as a child process of the app. "Host on
  this PC" installs it as a per-user service (`systemd --user` on Linux, Task Scheduler on
  Windows) with autostart and restarts. Logs and status are always visible in the app.
- **Alternatives:** only while the app runs; always a system-wide service.

### D-023 — Generated code is MIT-0; the user picks their bot's license
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Templates and generated code are licensed MIT-0, which needs no attribution,
  so the whole bot belongs to its user. When creating a project, the user may choose a
  license for their bot. By default there is none.
- **Alternatives:** `MIT OR Apache-2.0` (every bot would have to keep the notice); no
  license, with a statement in the README.

### D-024 — UI in English and Polish from the start, with i18n
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** All UI text goes through an i18n system from day one. English and Polish
  ship with the first version, and the system is ready for community translations,
  including as plugins or language packs. Which i18n library is still open. The Polish
  translation files are the only non-English content allowed in the repository (D-005).
- **Alternatives:** English only with i18n ready; English only, i18n later.

### D-025 — Linux distribution: native packages, RPM on Copr first
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The first package is an RPM in Copr for Fedora, then .deb, AUR, openSUSE
  and a tarball. Updates come through the package manager. Flatpak is not used first,
  because its sandbox conflicts with D-017 (installing system packages) and D-022
  (`systemd --user` services).
- **Alternatives:** AppImage + tarball with a built-in updater; Flatpak (Flathub); RPM +
  AppImage together.

### D-026 — Quality bar: gate + performance budgets
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Every code change passes `fmt`, `clippy -D warnings` and the tests, including
  gpui-kit's headless UI tests, with CI on Linux (Windows joins with D-009). Key paths have
  measured budgets: startup time, frame time, memory, and the speed of code generation and
  of builds. A stage is not done until its numbers are measured. The budget values are
  still open.
- **Alternatives:** the full djs-rs level (benchmarks and stress tests for every stage);
  a minimal gate without performance measurements.

### D-027 — The app builds with the latest stable Rust, with no MSRV
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The app uses edition 2024 and is always built with the latest stable Rust.
  It promises no MSRV, because users install packages rather than compile it, and gpui
  raises its requirements quickly. This applies to the app only. Bots are built by the
  private toolchain (D-017).
- **Alternatives:** a fixed MSRV bumped deliberately; an exact pinned version.

### D-028 — A multi-crate Cargo workspace from the start
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Separate crates from day one: the core (project model, no UI), the code
  generator, the toolchain manager, the runner/hosting, the plugin host, the plugin SDK,
  the integrations, and the UI app. Logic is testable without a window, and rebuilds are
  faster. The exact crate list is confirmed with the owner when the skeleton is built.
- **Alternatives:** one crate, split later; UI + core + SDK.

### D-029 — Crate prefix `rustbot-`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Crates are named `rustbot-*` (e.g. `rustbot-core`, `rustbot-codegen`,
  `rustbot-plugin-sdk`). `rustbot` itself is taken on crates.io, so the binary and the
  package are `rustbot-creator`. On 2026-11-17 `rustbot-creator`, `rustbot-core`,
  `rustbot-plugin` and `rustbot-sdk` were free. At least the plugin SDK will be published.
- **Alternatives:** `rbc-*`; `rustbot-creator-*`.

### D-030 — rust-analyzer comes as a rustup component
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The editor's LSP server is rust-analyzer, installed into the private
  toolchain (D-017) with `rustup component add rust-analyzer`, so it always matches the
  Rust version.
- **Alternatives:** download it from rust-analyzer's GitHub releases; no LSP at first.

### D-031 — Vertical slice first
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The first implementation phase is the smallest end-to-end path through every
  layer: the app window with the dock, a new project, a `/ping` command in the graph, code
  generation, a build with the private toolchain, and running the bot with its logs in the
  app. It validates the architecture before the scope widens.
- **Alternatives:** layer by layer; a UI prototype with mock data first.

### D-032 — The easy mode is widened first
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** After the slice, the easy mode (forms + graph) is widened first, because the
  advanced mode stands on the same graph (D-015) and the easy mode is the app's
  differentiator.
- **Alternatives:** the advanced mode first; both in parallel.

### D-033 — Scope of the first public release (Fedora)
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The first public release contains everything below. The order of phases
  within it is still open.
  - both modes and running the bot inside the app;
  - export as source code and as a ready build;
  - hosting on the user's PC in the background (`systemd --user`, D-022);
  - GitHub integration (device flow sign-in, creating a repository, pushing the project);
  - deploying to Cloudflare Containers (D-013);
  - JSON themes, with built-in dark and light themes (D-034).
- **Amended by D-123:** the first public release, on Fedora and Ubuntu, comes before
  the advanced mode, GitHub and Cloudflare Containers, which come in later releases.

### D-034 — Themes in the first release, WASM plugins after it
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** JSON themes, with built-in dark and light, ship in the first release. WASM
  plugins (D-016) come in the release after it. From day one the built-in nodes and panels
  are registered the same way plugins will register theirs, so adding the plugin system
  does not require a rewrite.
- **Alternatives:** both in the first release; both after it.

### D-035 — Phase order of the first release: the modes first, then the ways to ship
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** 2 vertical slice (workspace skeleton + CI first) → 3 easy mode → 4 advanced
  mode → 5 JSON themes + first run → 6 export → 7 hosting on the PC → 8 GitHub →
  9 Cloudflare Containers → 10 RPM on Copr + release 1. Each way to ship builds on the
  previous one: the export's build is what hosting and Cloudflare deploy. The details are
  in `ROADMAP.md`.
- **Alternatives:** the whole no-code path first, advanced mode last; interleaved, with
  local shipping early and the external services after the advanced mode.
- **Amended by D-123:** the order after phase 3 follows the incubation timeline.

### D-036 — Crates are created when their phase starts
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The vertical slice creates six crates: `rustbot-core` (project model,
  graph, node registry), `rustbot-codegen` (graph → Rust/djs-rs code), `rustbot-toolchain`
  (private rustup, system packages, `cargo build`), `rustbot-runner` (running the bot, logs),
  `rustbot-secrets` (keyring + encrypted file) and `rustbot-creator` (the gpui app).
  `rustbot-github`, `rustbot-cloudflare`, `rustbot-plugin-host` and `rustbot-plugin-sdk` are
  created in their own phases. No empty crates are kept in advance.
- **Alternatives:** the whole skeleton at once; fewer, larger crates in the slice.

### D-037 — i18n: Fluent for the app's text, rust-i18n inside gpui-kit
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The app's own text uses [Fluent](https://projectfluent.org/) (pure Rust),
  which handles Polish plural forms ("1 plik, 2 pliki, 5 plików") and is a format
  translators know. gpui-kit's components keep their own rust-i18n (YAML) strings, and the
  app supplies a Polish YAML for them.
- **Alternatives:** rust-i18n everywhere (one system, but no plural rules).

### D-038 — Performance budgets: balanced
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Measured on the owner's laptop (i7-1355U, 16 GB RAM, Iris Xe, KDE on
  Wayland), with bots in debug builds and dependencies already compiled:

  | Path | Budget |
  |---|---|
  | Startup to an interactive window | ≤ 500 ms |
  | Frame time (p99), graph of 500 nodes | ≤ 16.6 ms (60 FPS) |
  | Memory, one project open | ≤ 200 MB |
  | Code generation, 500 nodes | ≤ 100 ms |
  | Rebuild after a small change | ≤ 10 s |

- **Alternatives:** strict (250 ms, 120 FPS, 100 MB, 20 ms, 3 s); relaxed (1 s, 30 FPS,
  400 MB, 500 ms, 30 s).

### D-039 — C compiler: the system gcc, installed with consent
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** A C compiler is required to build bots in every setup. djs-rs uses `ring`
  for TLS, which compiles C, and rustc on Linux links through `cc`. If gcc is missing, the
  app installs it through the package manager behind the system's administrator prompt:
  `dnf install gcc` on Fedora, `build-essential` on Debian/Ubuntu, `base-devel` on Arch,
  `gcc` on openSUSE. The owner's machine has gcc 16.2.1 and cmake 4.3.0.
- **Alternatives:** a private zig with cargo-zigbuild (no admin rights, ~80 MB, pre-1.0);
  gcc if present, otherwise zig.

### D-040 — Generated code is committed in `generated/`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** A bot project's generated Rust code lives in `generated/` (D-015) and is
  committed. The bot's repository builds with plain cargo, without RustBot Creator, so
  there is no lock-in and GitHub Actions can build and deploy it. Code generation is
  deterministic, so diffs show only real changes. Layout v0:
  `project.toml` (settings, `schema = 1`, djs-rs version, license), `graphs/commands/*.json`,
  `graphs/events/*.json`, `src/` (the user's modules), `generated/` (`Cargo.toml`,
  `src/main.rs`, never edited by hand), and `.gitignore` (`target/`, `.env`).
- **Alternatives:** source only, with the generated code in a gitignored `.rustbot/` and a
  standalone project only through export.

### D-041 — Dependency set of the vertical slice
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Approved as a set:
  - already in gpui-kit's tree: `serde`, `serde_json`, `anyhow` (app-level errors),
    `smol` (async in the app), `tracing`;
  - new: `toml` 1.2 (D-019), `thiserror` 2 (typed errors in library crates),
    `fluent-bundle` 0.16 + `unic-langid` 0.9 (D-037), `keyring` 4.2 (D-020; on Linux its
    default store is the pure-Rust zbus Secret Service), `directories` 6 (XDG paths,
    D-017), `ureq` 3 + `sha2` 0.11 (downloading `rustup-init` and verifying its checksum),
    `tracing-subscriber`;
  - dev-only: `insta` (snapshots of generated code), `tempfile`.
  The encrypted-file fallback for secrets (D-020) and its crypto come in a later phase,
  since Fedora KDE has KWallet. Any dependency outside this set still needs approval.
- **Alternatives:** approve each dependency separately.

### D-042 — Our own node-graph canvas on gpui-kit's gpui
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** The easy mode's graph editor is written in this project, on the gpui
  version gpui-kit pins (`gpui-pre` 0.3.12 at gpui-kit 0.10.0). This gives full control
  over UX and the D-038 budget (500 nodes at 60 FPS). The existing crates cannot share a
  window with gpui-kit: `ferrum-flow` 0.3.1 is on `gpui` 0.2, and `gpui-node-graph` 0.1.0 is
  on `gpui-unofficial`.
- **Alternatives:** port `ferrum-flow` (Apache-2.0) as a fork; ask its author to support
  gpui-kit's gpui and wait.

### D-043 — Start Phase 2; plan each later phase when it starts
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Implementation starts with Phase 2 (the vertical slice). Each later phase
  (the node catalog, the GitHub scope, other hosts, …) is planned in detail at its start,
  using what the working slice has shown.
- **Alternatives:** plan phases 3–10 in detail before writing code.

### D-044 — Window layout: classic VS Code
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** An activity bar on the left, a side panel (the project tree), tabbed
  editors in the centre (graphs, files), a bottom panel (logs, problems) and a status bar
  (bot state, Rust version, branch, language). It is all built on gpui-kit's dock, so users
  can rearrange it.
- **Alternatives:** the same with a permanent right-hand properties panel for the selected
  node.

### D-045 — CI: Fedora 44 and Ubuntu from the start
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** GitHub Actions runs the gate as a matrix: a `fedora:44` container (the
  priority platform, D-009) and `ubuntu-latest`. The repository is public, so the minutes
  are free.
- **Alternatives:** the `fedora:44` container only; `ubuntu-latest` only.

### D-046 — Lints as in djs-rs
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `unsafe_code = "deny"`. Clippy `all` + `pedantic` warn, and CI runs with
  `-D warnings`. `missing_docs` applies in library crates but not in the app, and
  `missing_debug_implementations`, `unreachable_pub` and `rust_2018_idioms` are on.
- **Alternatives:** milder (clippy `all` only, no docs requirement); stricter (+ clippy
  `nursery`, `missing_docs` everywhere).

### D-047 — Data in the graph: text templates + data pins
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Text fields accept simple templates, as popular bot panels do
  (`Banned {target.name}`). Computations and complex logic use data pins wired between
  nodes. Both compile to typed Rust and are checked before the build. Execution order is a
  separate kind of wire (flow).
- **Alternatives:** data pins only (Blueprints-like); templates/variables only, with wires
  for order only.

### D-048 — Node positions live in a separate layout file
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Logic is in `graphs/<kind>/<name>.json`, and canvas positions are in
  `graphs/<kind>/<name>.layout.json`. Moving a node does not touch the logic file, so diffs
  and reviews show only real changes.
- **Alternatives:** positions inline in the graph file.

### D-049 — Gateway intents are computed from the graph
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Code generation derives the intents from the nodes used, so a beginner
  never has to know about them. `project.toml` stores only extra intents added by hand
  (advanced mode). The app warns about privileged intents (e.g. `MESSAGE_CONTENT`) that
  must be enabled in the Discord Developer Portal.
- **Alternatives:** intents chosen by hand in `project.toml`.

### D-050 — A random UUID identifies a project (extends D-041 with `uuid`)
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `project.toml` has an `id`, a random UUID v4 generated once when the
  project is created. Secrets in the keyring are keyed by it (D-020), so it survives moving
  or renaming the folder. The `uuid` crate (1.29.3) is already in gpui's dependency tree,
  so it adds nothing to the build. It is added to the approved set of D-041.
- **Alternatives:** key secrets by the bot's package name (collisions, lost on rename) or
  by the folder path (lost on move).

### D-051 — Project format v0
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** Approved as shown:

  ```toml
  # project.toml
  schema = 1
  id = "3f2b6c1e-8a4d-4f7e-9b1a-2c5d7e8f9a0b"
  [bot]
  name = "Ping Bot"
  package = "ping-bot"
  description = "Replies to /ping."
  license = ""            # SPDX expression, empty = none (D-023)
  [djs-rs]
  version = "0.1.0-alpha.1"
  features = []
  [gateway]
  extra-intents = []      # the rest come from the graph (D-049)
  ```

  ```json
  // graphs/commands/ping.json
  { "schema": 1,
    "nodes": {
      "n1": { "type": "core.slash_command",
              "props": { "name": "ping", "description": "Replies with Pong!" } },
      "n2": { "type": "core.reply", "props": { "content": "Pong, {user.name}!" } } },
    "edges": [ { "from": "n1.then", "to": "n2.run" } ] }
  // graphs/commands/ping.layout.json
  { "schema": 1, "nodes": { "n1": [0, 0], "n2": [280, 0] } }
  ```

  - Nodes are a map from id to node, with sorted keys for deterministic diffs. The app
    assigns ids (`n1`, `n2`, …), and they never change.
  - Node types are namespaced: `core.` for built-ins, a plugin's id for plugin nodes
    (D-034).
  - There is one list of edges, `from`/`to` as `node.pin`. The node registry knows whether
    a pin is flow or data.
  - Templates use `{path}`, and `{{` and `}}` give literal braces, as in Rust's `format!`.
- **Alternatives:** readable, editable node ids (`command`, `reply`).

### D-052 — Node registry: node types are Rust structs (data only)
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** A node type is plain data: an id (`core.reply`), a category (Trigger,
  Action, Logic, Data), pins (flow/data, in/out), props (form fields, with whether text
  takes templates), the variables a trigger provides, and the intents it needs (D-049).
  Built-ins are defined in Rust and registered through the same `Registry::register` that
  WASM plugins will use with the same structs serialized (D-034). The registry validates
  graphs for the "Problems" panel. Labels are Fluent keys derived from the id
  (`node-core-reply`, `node-core-reply-content`, D-037). The Rust code for each node type
  comes from `rustbot-codegen`, not from the registry.
- **Alternatives:** declarative TOML files per node type.

### D-053 — Template variables have short names
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** For example `{user.name}`, `{user.mention}`, `{channel.name}`,
  `{server.name}`, `{option.<name>}`. They are short and beginner-friendly, like popular
  bot panels.
- **Alternatives:** djs-rs/discord.js paths (`{interaction.user.username}`).

### D-054 — `{user.name}` is the name people see in the chat
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `{user.name}` is the server nickname if there is one, otherwise the global
  display name, otherwise the username. The exact values are separate:
  `{user.username}`, `{user.display_name}`, `{member.nickname}`.
- **Alternatives:** the username.

### D-055 — Slash commands: a test server while working, global when shipped
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** When the bot runs from the app, its commands are registered on a test
  server the user picks, so they appear at once. Export, hosting and deploys register
  them globally. This is how djs-rs's own example works.
- **Alternatives:** always global; a per-project setting.

### D-056 — The generated bot reads its settings from the environment, or `.env`
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `DISCORD_TOKEN` and `DISCORD_TEST_GUILD_ID`. If the guild id is set, the
  commands are registered on that server; if not, they are registered globally (D-055).
  The runner passes both from the keyring and never writes them to disk. Anyone running an
  exported bot by hand can use a `.env` file instead, which is gitignored (D-040). This is
  how the djs-rs examples work.
- **Alternatives:** environment variables only.

### D-057 — Generated code: one module per graph
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `generated/src/main.rs` holds the client, the intents and the command
  registration. `generated/src/commands/<name>.rs` and `generated/src/events/<name>.rs` hold
  one module per graph. This is readable after export, and editing one graph changes one
  file.
- **Alternatives:** a single `main.rs`.

### D-058 — Code is generated as tokens: `quote` + `prettyplease` (extends D-041)
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `rustbot-codegen` builds code with `quote`/`proc-macro2` (already in the
  tree) and formats it with `prettyplease` 0.3 on `syn` 3. Project content, which may come
  from a cloned repository, then cannot escape a string literal into code that is compiled
  and run. The output is formatted deterministically without rustfmt. These crates are
  added to the approved set of D-041.
- **Alternatives:** text templates with `{:?}` escaping; text templates + rustfmt from the
  private toolchain.

### D-059 — Dependencies of the generated bot
- **Date:** 2026-11-17 · **Decided by:** Owner
- **Decision:** `djs-rs`, `tokio` (macros, multi-threaded runtime) and `tracing-subscriber`.
  The last prints djs-rs's logs (connection, errors, rate limits), and the runner shows
  them in the Logs panel, as in the djs-rs examples.
- **Alternatives:** `djs-rs` + `tokio` only.

### D-060 — Fully local, no accounts
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** The app works 100% locally, and nobody needs an account to use it: there
  is no RustBot Creator account, sign-in, telemetry or cloud backend. The internet is used
  only where a feature cannot work without it: installing packages (the toolchain, crates,
  system packages), hosting and deploys, external integrations (GitHub, Cloudflare and
  later ones), and running the bot itself, which talks to Discord. Each of these is
  optional and started by the user. Templates, node types, themes, translations and docs
  ship with the app.
- **Amended by D-125:** an account exists only inside the optional RustBot Cloud.

### D-061 — App id `io.github.kotoanidev.RustBotCreator`
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** The reverse-DNS id for the desktop file and D-Bus. Data and cache live in
  `~/.local/share/rustbot-creator` and `~/.cache/rustbot-creator`.
- **Alternatives:** an id from the owner's own domain.

### D-062 — The bots' Rust version is pinned per app release
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** Each app release installs one tested Rust version into the private
  toolchain, now `1.100.0` (`rustbot_core::RUST_VERSION`), and only the next release bumps
  it. `generated/rust-toolchain.toml` pins it too, so a hand build of an export uses the
  same version.
- **Alternatives:** always the latest stable; djs-rs's MSRV (1.89).

### D-063 — Bots share one build cache
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** One `CARGO_TARGET_DIR` at `~/.cache/rustbot-creator/target`, so djs-rs and
  the other dependencies compile once for every bot. Right after a build, the binary is
  copied into that bot's own folder, so two bots with the same package name never
  overwrite each other.
- **Alternatives:** a separate `generated/target` per bot.

### D-064 — Project trust: ask before the first build of a project from elsewhere
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** This works like VS Code's Workspace Trust. Projects created in the app are
  trusted. A project from elsewhere (cloned, downloaded) opens for viewing and editing,
  and its first "Build/Run" shows what it contains (the user's own code, dependencies)
  and asks for one confirmation, which is remembered for that project.
- **Alternatives:** no prompt (as plain cargo); a prompt before every build of a project
  from elsewhere.

### D-065 — The test server id is the user's, outside the project
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** The test server (D-055) is stored in the app's data folder and keyed by the
  project id, like the token. A cloned project imposes nobody's server, and changing it
  never touches git.
- **Alternatives:** in `project.toml`, shared with collaborators.

### D-066 — Node properties in a side-bar panel
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** The selected node's form is a "Properties" panel under the project tree in
  the side bar, like VS Code's Outline under the Explorer. It is a dock panel, so it can be
  moved or closed. The canvas stays wide, and long fields such as a reply's content have
  room.
- **Alternatives:** fields directly on the nodes; a form dialog opened by double-click.

### D-067 — Adding nodes: a searchable list, opened three ways
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** A searchable node list opens at the cursor on a right-click on the canvas,
  from a visible "+" button for beginners, and when a wire is dropped on empty canvas,
  where it shows only nodes that fit that pin. It is one implementation, as in Unreal
  Blueprints and ComfyUI.
- **Alternatives:** a palette panel to drag from; both.

### D-068 — Autosave
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** Every change is saved by itself after a short pause, and undo (Ctrl+Z)
  works inside the app. The user still makes git commits.
- **Alternatives:** explicit Ctrl+S with dirty markers; autosave that can be switched off
  in the settings (Phase 5).

### D-069 — New projects use djs-rs 0.1.0-alpha.2
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** New projects pin `=0.1.0-alpha.2` (`rustbot_core::DJS_RS_VERSION`). This
  release fixed [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP
  interactions, wasm32). Existing projects keep the version in their `project.toml` until
  the user changes it; an "update djs-rs" action comes later. The generated `/ping` bot
  builds on alpha.2 unchanged, and its breaking change (the gateway behind the `gateway`
  feature) does not touch bots with default features (C-002).
- **Alternatives:** also bump existing projects when they are opened; stay on alpha.1
  until the Workers phase.

### D-070 — Headless UI tests with gpui-kit's `test-support`
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** `rustbot-creator` enables gpui-kit's `test-support` feature in its
  dev-dependencies only. Tests render the real views in a hidden window and simulate
  clicks and keys, so the canvas, the forms and Run are checked in CI. The extra crates it
  pulls never reach the app.
- **Alternatives:** unit tests of the logic only, with manual checks in the window.

### D-071 — Run/Stop in the status bar
- **Date:** 2026-11-22 · **Decided by:** Owner
- **Decision:** "▶ Run" / "■ Stop" sits on the left of the status bar, next to the bot's
  state. It is always visible and takes no room.
- **Alternatives:** a toolbar above the editor; a button in a custom title bar.

### D-072 — The bot detects that it is not on the test server
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Context:** the owner's first live run failed with 403: Discord refuses to register
  commands in a server the bot is not on (`50001 Missing Access`), and the raw error did
  not say what to do.
- **Decision:** on start, the generated bot checks the server list Discord sends with
  Ready. If the test server is not on it, the bot does not try to register its commands.
  It prints a plain message to the Logs, in English like the rest of the bot's output,
  with an invite link that has the `bot` and `applications.commands` scopes and the server
  preselected.
- **Alternatives:** an "Invite the bot" button in the bot settings, which needs the
  application ID; both; only instructions.

### D-073 — Phase 3 starts with the `/ban` example of D-014
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The first nodes of the catalog make the D-014 example work end to end:
  - **Command options:** user, text, number, channel, role.
  - **Condition:** yes/no branches, e.g. "has permission?" or a comparison.
  - **Moderation:** ban, kick, timeout, roles.
  - **Messages:** send to a channel, a reply only the author sees, an embed.

  This crosses every category: trigger, logic and actions.
- **Alternatives:** a welcome bot (events first); buttons and menus; a little of
  everything.

### D-074 — Nodes take values from form fields with a variable picker
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A value such as "which user to ban" is a form field on the node. The field
  offers the variables that fit its type, such as `{option.target}` or `{user}`, in the
  spirit of D-053. No extra wires are needed. The data pins of D-052 stay in the model,
  unused by the easy mode for now.
- **Alternatives:** data pins wired by cables; both (a field, or a connected data pin).

### D-075 — A small console step opens Phase 3
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Before the new nodes, the Logs panel gets:
  - selecting and copying text;
  - level colours (ERROR red, WARN yellow);
  - a "Clear" button;
  - build lines told apart from the bot's lines.

  A full log viewer can come later.
- **Alternatives:** a full log viewer with Phase 7 (hosting on the PC); both.

### D-076 — Command options are a list in the command's form
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The slash command's properties have an "Options" list with "+ Add
  option". Each option has a name, a type (text, number, yes/no, user, channel, role), a
  description and "required". An option becomes the variable `{option.<name>}`, and the
  graph stays uncluttered.
- **Alternatives:** a small node per option, attached to the command.

### D-077 — One node per condition, with "yes" and "no" outputs
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Conditions are separate node types in the node list: "Has permission?",
  "Has role?" and "Compare". Each has a "run" input and two flow outputs, "yes" and "no".
  They are easy to find with search, and their forms stay simple.
- **Alternatives:** one "If" node whose form changes with the kind of condition.

### D-078 — The variable picker: a list for typed fields, {x} for text
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Fields of type user, channel or role are drop-down lists of the variables
  that fit, e.g. "Option: target" or "Command author". Text fields have an {x} button that
  inserts the chosen variable at the cursor.
- **Alternatives:** suggestions after typing `{`; both.

### D-079 — The first embed is basic
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Title, description, colour, image and footer, each taking variables. More
  fields come with the full catalog.
- **Alternatives:** full (fields, author, thumbnail, title link, timestamp).

### D-080 — A role field also takes a role ID
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A role field's list holds the role variables and, last, "Role with ID…".
  That one takes an ID copied from Discord (developer mode → right-click the role → Copy
  ID). It works offline, and the ID survives renaming the role.
- **Alternatives:** variables only (no fixed role for now); the app fetches the test
  server's roles with the bot token (the app itself would talk to Discord, D-060).

### D-081 — "Compare" ignores letter case
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** For "equals" and "contains", "Spam", "SPAM" and "spam" are the same.
  Numbers are compared as numbers.
- **Alternatives:** exact comparison; a "match case" switch on the node.

### D-082 — A moderation action that fails continues at "error"
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Ban", "Kick", "Timeout", "Add role" and "Remove role" have two outputs:
  "then", when the action worked, and "error", when it failed. It can fail when the bot
  lacks the permission, the target's role is above the bot's, or the command came from a
  DM. Wiring "error" lets the bot answer, e.g. "I can't do that". If "error" is not
  wired, the failure goes to Logs and the rest of the command does not run.
- **Alternatives:** the command stops, and Discord shows "The application did not
  respond"; an automatic reply only the author sees.

### D-083 — "Ban" can delete recent messages
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Ban" has a field "Delete messages from the last" with the choices "don't
  delete", 1 hour, 24 hours and 7 days. "Don't delete" is the default.
- **Alternatives:** a ban never deletes messages.

### D-084 — A timeout's length is minutes, as a number or a variable
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Timeout" has a field "Minutes" that takes a number or a variable, e.g.
  `{option.minutes}`, so a `/timeout` command can ask for the length. Discord allows at
  most 28 days (40 320 minutes). Problems flags a larger number that is typed in, and one
  from a variable takes the "error" output (D-082).
- **Alternatives:** a fixed list (1 minute, 5 minutes, 10 minutes, 1 hour, 1 day, 1 week).

### D-085 — Moderation commands are gated by Discord and warned about
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A command with a moderation action gets Discord's default member
  permissions for it, e.g. "Ban members" for "Ban". Members without that permission do not
  see the command, though a server admin can change this under Integrations. Problems also
  warns when an action is not behind "Has permission?" for the command author. The warning
  does not block Run.
- **Alternatives:** only the Discord gate; only the warning; a "Who can use it" field on
  the command, set to "everyone" by default.

### D-086 — "Only the author sees it" is a switch on "Reply"
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Reply" gets a checkbox "Only the command author sees it" (an ephemeral
  reply). It is off by default.
- **Alternatives:** a separate "Reply privately" node.

### D-087 — An embed is a section of the message form
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Reply" and "Send to a channel" have a collapsible "Embed" section with
  the D-079 fields. The embed is sent when its title or description is not empty, and the
  message text may then be empty.
- **Alternatives:** an "Embed" node whose data output wires into the message.

### D-088 — A channel field also takes a channel ID
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Like roles (D-080), a channel field lists the channel variables and, last,
  "Channel with ID…". That lets moderation logs go to a fixed channel, for example.
- **Alternatives:** variables only.

### D-089 — Problems warns when a command may not reply
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Discord needs an answer to a command within 3 seconds, or it shows "The
  application did not respond". Problems warns, without blocking Run (D-085), when a path
  through a command can end without "Reply".
- **Alternatives:** the bot sends a private "✅" itself; nothing.

### D-090 — Problems are shown on the graph next
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The next Phase 3 item is showing problems on the graph. A node with a
  problem gets a red border, and one with a warning a yellow one. Clicking a problem in
  the Problems panel selects its node and shows its form. The full node catalog comes
  after this.
- **Alternatives:** events first; buttons and menus first; bot templates first.

### D-091 — The catalog starts with events
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The first part of the full node catalog is events: graphs in
  `graphs/events/` started by an event, not a command. The first events are: a member
  joined, a member left, a new message, the bot is ready, and a reaction added or removed.
  The app explains the privileged intents that some of them need (Server Members,
  Message Content), which are switched on in the Discord Developer Portal.
- **Alternatives:** buttons and menus; more command actions; bot templates.

### D-092 — New graphs come from "+" in the project tree
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Commands" and "Events" in the project tree each have a "+" button. A new
  command asks for its name, e.g. `ban`. A new event offers the list of events, and its
  file is named after the event, e.g. `member_join`. A graph is deleted by right-clicking
  it, choosing "Delete" and confirming.
- **Alternatives:** a right-click menu on the sections; a "New…" button in the panel's
  toolbar.

### D-093 — "New message" skips bots unless asked
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The "New message" trigger has a checkbox "Also messages from bots", off by
  default. The bot's own messages are always skipped, so it cannot loop by answering
  itself.
- **Alternatives:** always skip bots.

### D-094 — In a message event, "Reply" answers that message
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** In a "New message" graph, "Reply" sends a Discord reply to that message.
  In other events Problems says to use "Send to a channel" instead. A new "Delete message"
  node deletes the message, for automatic moderation.
- **Alternatives:** "Reply" only in commands, with "Send to a channel" in events.

### D-095 — Privileged intents are explained in the form and in Logs
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** An event that needs a privileged intent (Server Members for members
  joining or leaving, Message Content for a message's text) shows a note in its form. The
  note says to switch the intent on in the Developer Portal under Bot, with a link. If
  Discord refuses the intents, Logs show a plain message with the link (as in D-072).
- **Alternatives:** a warning in Problems, which stays even once the intent is on; only
  Logs.

### D-096 — Buttons and menus come next in the catalog
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After the events, the catalog continues with buttons and select menus
  under messages.
- **Alternatives:** more actions (edit or pin a message, nicknames, channels); bot
  templates; waiting for the owner's live test first.

### D-097 — A click continues the same graph, in one of two modes
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** What a button or menu does is wired from the message node's outputs, in
  the same graph. Two modes:
  - **Always works** (the default): the click is routed by djs-rs (`client.component`),
    so buttons keep working after the bot restarts. After the click, only the clicker,
    the channel and the server are known; earlier variables are a problem there.
  - **Wait for a click:** a "Wait for a click (minutes)" field makes the bot wait for
    the click in the same handler (`await_component`). Every earlier variable stays
    usable, which suits confirmations (`/ban` → "Sure?" → Yes → Ban). An output "time
    ran out" follows when nobody clicks in time. These buttons stop working after the
    time or a restart.
- **Alternatives:** only one of the two modes; a separate "Button clicked" graph linked by
  the button's ID.

### D-098 — Buttons are a section of the message form
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Reply" and "Send to a channel" get a folding "Buttons" section, like
  "Embed" (D-087). It holds up to 5 buttons, each with a label and a colour, or a link.
  Each button other than a link adds an output "clicked: <label>" to the node.
- **Alternatives:** a separate "Button" node wired to the message.

### D-099 — A select menu has its own options, one output each
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A message can carry a select menu with options the user types in. One
  option is picked at a time, and each option has its own output (e.g. "Red" → Add
  role).
- **Alternatives:** one "picked" output with a `{choice}` variable and several picks;
  Discord's user, role and channel menus as well (later).

### D-100 — Anyone can click, unless "only the command author" is ticked
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** By default anyone can click a button or use a menu (verification, role
  menus). A checkbox "Only the command author can click" limits it, for confirmations;
  others get a private reply saying the button is not theirs.
- **Alternatives:** always anyone, with a manual "Compare" on the user's ID.

### D-101 — More actions come next in the catalog
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After buttons and menus, the catalog continues with more actions.
- **Alternatives:** modals; the bot's memory (lasting variables); bot templates.

### D-102 — The actions of this round
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:**
  - "Send a DM" to a user, with the same message form as "Send to a channel" (embed,
    buttons, menu).
  - "Add a reaction", "Pin" and "Unpin".
  - "Edit message": new text and embed for a message the bot sent.
  - "Change nickname", a moderation action needing "Manage nicknames", and "Wait", which
    waits some seconds and goes on.
- **Alternatives:** any subset of these.

### D-103 — Message actions work on the event's message or the last one sent
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Add a reaction", "Pin", "Unpin" and "Edit message" pick their message
  from a list: "The event's message" ("New message", reactions) or "Last sent by the
  bot" in this run of the graph.
- **Alternatives:** picking an earlier message node of the graph; also a message by its
  ID and channel.

### D-104 — The bot's memory comes next in the catalog
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After the actions of D-102, the catalog continues with the bot's memory:
  variables that last across runs and restarts.
- **Alternatives:** modals; bot templates; a pause for the owner's live test.

### D-105 — Memory is a JSON file in the bot's folder
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The running bot keeps its memory in one `data.json` in its folder, held in
  memory and written atomically after each change. The generated bot gets `serde_json`
  as a direct dependency, which djs-rs already brings. Hosting without a disk (e.g.
  Cloudflare) will need another store later.
- **Alternatives:** the embedded database redb; SQLite through rusqlite.

### D-106 — Memory variables are declared in a "Memory" list
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The project tree gets a "Memory" branch with "+". Each variable has a name,
  a type (number or text), a scope and a starting value, kept in `project.toml`. Nodes
  pick a variable from a list, so a typo cannot create a new one.
- **Alternatives:** a variable comes into being when a node first names it.

### D-107 — Four scopes
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A memory variable is global (one for the bot), per server (e.g. a log
  channel), per user (e.g. a balance) or per user on a server (e.g. warnings, XP).
- **Alternatives:** only global and per user on a server, for now.

### D-108 — "Set", "Change by" and "Reset", read as `{memory.name}`
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Three nodes: "Set" (a value or a template), "Change by" (add or subtract a
  number, e.g. one more warning) and "Reset" (back to the starting value). Templates and
  "Compare" read a variable as `{memory.name}`.
- **Alternatives:** also a "Ranking" node: the top 10 users by a number variable.

### D-109 — Memory variables are made in the node
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** This replaces the "+" and the separate form of D-106, which the owner found
  bad and unused.
  - A memory node's "Variable" list ends with "New variable…", which asks for a name.
    Enter makes the variable and picks it.
  - The type, scope and starting value of the picked variable are edited in that node's
    form, and the change applies to every node that uses it.
  - The tree's "Memory" branch lists the variables, each with the nodes that use it, and
    clicking a node opens it there.
  - Right-clicking a variable renames it, which also fixes every node and template that
    uses it, or deletes it.
- **Alternatives:**
  - a "Memory" panel in the dock with a table of the variables;
  - removing memory from the app until a better design.

### D-110 — Modals come next in the catalog
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After the bot's memory, the catalog continues with modals: a pop-up form
  that a command or a click opens.
- **Alternatives:** bot templates; the rest of the catalog (channels, threads, roles,
  "Ranking"); phase 4, the advanced mode.

### D-111 — A modal is waited for in the same graph
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A "Show form" node waits for the form to be sent, up to 15 minutes. The flow
  then goes on with the typed values and every variable of the run. A "timeout" output
  runs if nothing was sent in time; Discord does not report a closed form.
- **Alternatives:** a form that always works, handled on its own like the clicks of D-097;
  both, switched in the node.

### D-112 — Modals have text fields only
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A form has 1–5 text fields, each short (one line) or long (a paragraph),
  with a label, "required", a hint and a minimum and maximum length.
- **Alternatives:** also radio groups, checkboxes and menus; all of that and file uploads.

### D-113 — A form opens after a command or a click
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Show form" may follow a slash command or a click of a button or menu,
  before the bot answers it in any way: Discord allows a form only as the first answer.
  Validation checks the order.
- **Alternatives:** only after a command.

### D-114 — Typed values are read as `{field.<name>}`
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Each field has a name, with the rules of a command option's name, and a
  label that users see. After the form is sent, a field's value is `{field.<name>}`.
- **Alternatives:** numbered fields, `{field.1}`.

### D-115 — Bot templates come next, chosen when a project is made
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After forms, phase 3 continues with bot templates. "New project" shows the
  templates as cards with a description, "Empty" (with `/ping`) among them, and then the
  folder dialog.
- **Alternatives:**
  - for the timing: the rest of the catalog first; phase 4;
  - for the choice: also "Add from template" in the project tree; only that.

### D-116 — Four templates to start
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:**
  - "Moderation with warnings": `/ban`, `/kick` and `/timeout`, each checking the
    permission, and `/warn`, which counts warnings in memory and shows the count.
  - "Welcome": a message in a chosen channel when someone joins or leaves, and a welcome
    DM.
  - "Roles from a menu": `/roles` sends a message with a select menu, and picking an
    option gives or takes a role; the clicks always work.
  - "Reports by form": `/report` opens a form and sends the report to the moderators'
    channel, answering privately.
- **Alternatives:** levels with XP for messages.

### D-117 — Channels and roles a template needs are left empty
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A template leaves the fields for the user's channels and roles empty. The
  Problems panel lists them as empty fields, and a click opens the node to fill in the ID.
- **Alternatives:** a short wizard asking for the IDs when the project is made.

### D-118 — Templates speak the app's language
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** A template's bot texts (replies, command and option descriptions, labels)
  are in Polish when the app is in Polish, else in English. They live in the Fluent
  files with the UI text (D-024). Command and option names stay English, e.g. `/ban`.
- **Alternatives:** always English; a language switch on the template's card.

### D-119 — The rest of the catalog: channels, threads, roles, a ranking
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Phase 3 closes with four groups of nodes:
  - "Create channel" (a text channel, in a category if chosen) and "Delete channel";
  - "Create thread" (from the event's or the last message, or in a channel) and "Add to
    thread";
  - "Create role" (name, colour) and "Delete role";
  - "Ranking" (the top users by a number variable of memory), with a "Levels" template:
    XP for messages, `/level` and `/top`.
- **Alternatives:** phase 4, the advanced mode; phase 5, themes and the first run; a
  pause for the owner's live test.

### D-120 — What a node creates is a variable after it
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After "Create channel", "Create thread" or "Create role", the rest of the
  run knows what was created: `{created_channel.mention}` and the like in templates, and
  "Created channel" in the channel, thread and role pickers. A ticket can be created and
  written to at once.
- **Alternatives:** nodes that only create; a later node needs the ID typed in.

### D-121 — A new channel is visible as its category, or private
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** "Create channel" has a choice: "As the category or server" keeps
  Discord's permissions. "Private" lets only a chosen user, a chosen role (e.g. the
  moderators) and the bot see it, which is enough for tickets.
- **Alternatives:** always as the category; privacy only by a category set up on
  Discord.

### D-122 — "Ranking" gives its list as text
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After "Ranking", `{ranking}` holds ready lines such as "1. @who — 120":
  the top 1–25, with mentions that notify nobody. It goes into any message or embed.
- **Alternatives:** a node that replies with a ready embed itself.

### D-123 — The phase order follows the incubation timeline (amends D-035)
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** After phase 3, the app ships to users before the advanced mode, as the
  incubation application promises an MVP in about 6 months. The order is:
  1. 5 first run + themes;
  2. 6 export;
  3. 7 hosting on the PC;
  4. a public Linux release (RPM on Copr and .deb);
  5. Windows;
  6. a pilot of RustBot Cloud;
  7. 4 advanced mode;
  8. 8 GitHub.

  The phases keep their numbers as names. Cloudflare Containers (phase 9) was not placed
  by the proposal and stays after GitHub, as before. So the first public release (D-033)
  no longer waits for the advanced mode, GitHub and Cloudflare: they come in later
  releases.
- **Alternatives:** keep D-035 (the advanced mode next).

### D-124 — Windows right after the public Linux release (amends D-009)
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Windows comes right after the public release on Fedora and Ubuntu, before
  the RustBot Cloud pilot and the advanced mode (months 4–6 of the timeline). Fedora stays
  the priority platform and the test environment.
- **Alternatives:** D-009 unchanged: Windows after everything on Linux.

### D-125 — An account only inside RustBot Cloud (amends D-060)
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The app stays 100% usable with no account. An account exists only inside
  the optional RustBot Cloud feature, which the user starts, the same way GitHub and
  Cloudflare sign-ins work. The app still has no telemetry. Product metrics (sign-ups,
  deploys, subscriptions, churn) come only from the Cloud service's own side.
- **Alternatives:** D-060 unchanged until the Cloud pilot.

### D-126 — The business notes stay out of the public repository
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** `docs/BUSINESS.md`, the product direction with prices, forecasts and the
  incubation application, is kept only on the owner's machine. It is listed in
  `.git/info/exclude`, so it is never committed. Sessions on that machine read it.
- **Alternatives:** commit it as it is; commit a shortened version without prices,
  forecasts and the application.

### D-127 — Themes in gpui-component's JSON format (details D-034)
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Themes use the JSON theme-set format of gpui-component, which gpui-kit is
  built on. The app ships a dark and a light theme and can follow the system. The user's
  own themes are loaded from `~/.config/rustbot-creator/themes` as soon as they change, so
  gpui-component's existing themes work unchanged.
- **Alternatives:** the app's own JSON schema, translated into gpui-kit's colours.

### D-128 — A first-run wizard on the first start
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** The first start opens a wizard with these steps:
  1. language and theme;
  2. installing Rust and gcc, with consent (D-017, D-039);
  3. the bot in the Discord Developer Portal (D-129);
  4. a first project from a template.

  Each step can be skipped, and the wizard can be opened again from Settings.
- **Alternatives:** a wizard when the first project is made; no wizard, with only a
  Developer Portal guide in the bot settings.

### D-129 — A Developer Portal guide whose steps are checked
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** Each step has a short text and a button that opens the right page of the
  portal. The app checks the token by asking Discord who the bot is, makes the invite
  link with the permissions the bot needs, and checks that the bot is on the test server.
  There are no screenshots of Discord.
- **Alternatives:** text and links only, with no checks; the same with pictures of the
  portal, which need updating whenever Discord changes it.

### D-130 — Settings as an editor tab, in `settings.toml`
- **Date:** 2026-11-23 · **Decided by:** Owner
- **Decision:** As in VS Code, Settings open as a tab in the editor and are saved in
  `~/.config/rustbot-creator/settings.toml`. The first settings are:
  - language;
  - theme;
  - font size;
  - the toolchain: its state, its disk use, and installing it again;
  - opening the first-run wizard again.
- **Alternatives:** a separate dialog with tabs, using the same file.

---

## Defaults chosen by Claude (reviewable)

### C-001 — Repository housekeeping
- **Date:** 2026-11-17
- **Choice:** `.editorconfig` and the license texts are taken from djs-rs, with the copyright
  holder "The RustBot Creator contributors". `.gitignore` covers the Rust build output,
  editor files and `.env` secrets. There is no Cargo workspace, toolchain file or CI yet,
  because their shape depends on open questions below.

### C-002 — Generated bots keep djs-rs's default features
- **Date:** 2026-11-17
- **Choice:** Generated bots use djs-rs's default `zstd-stream` gateway compression rather
  than the pure-Rust `zlib-stream`. A C compiler is required anyway (D-039), so the
  pure-Rust option saves nothing, and zstd is djs-rs's own default.

### C-003 — How project format v0 is read and written
- **Date:** 2026-11-17
- **Choice:** Every file is written to a temporary file and then renamed, so a crash
  never leaves half a file. Unknown fields are rejected, so a typo in a hand-edited
  `project.toml` is an error rather than silently ignored. Any `schema` other than 1 is
  rejected until migrations exist (open question). `package` is derived from the name:
  ASCII letters and digits are kept, everything else becomes `-`, there is no
  transliteration, and the user can edit it. `Project::create` refuses a folder that
  already has `project.toml`, and writes `.gitignore` only if none exists. Maps are
  `BTreeMap`, because gpui's dependency tree turns on serde_json's `preserve_order`.

### C-004 — Details of the generated bot
- **Date:** 2026-11-22
- **Choice:** djs-rs is pinned with `=` (`=0.1.0-alpha.1`), because alphas may break each
  other. `generated/Cargo.toml` has an empty `[workspace]`, so a bot inside another cargo
  workspace still builds. Commands are registered on `ClientReady`. Ctrl+C shuts the
  client down cleanly. Module names come from graph file names: lowercase ASCII, other
  characters become `_`, a leading non-letter gets `command_`, and keywords get
  `_command`; two graphs that map to one name are an error. `write` removes files in
  `generated/src` that the project no longer generates. Variables are read from
  `cmd.interaction()`: the router's `CommandInteraction` derefs to the command data, whose
  `guild_id` is where the command is registered, not where it was used. A test
  (`generated_bot_compiles`, its own CI job) builds the generated bot against djs-rs from
  crates.io.

### C-005 — How the private toolchain installs and builds
- **Date:** 2026-11-22
- **Choice:** rustup-init comes from `static.rust-lang.org` and is checked against its
  `.sha256` before it runs. It installs with `--no-modify-path --profile minimal`, with
  `RUSTUP_HOME` and `CARGO_HOME` in the data folder. Every rustup or cargo command sets
  `RUSTUP_TOOLCHAIN`, which outranks any `rust-toolchain` file on the way. Builds are debug
  builds (D-038). The binary is copied, then renamed, to
  `~/.local/share/rustbot-creator/bots/<project id>/<package>`, in the data folder rather
  than the cache because the background service (D-022) runs it from there. Cargo runs in
  the cache folder with `--manifest-path`, because it reads `.cargo/config.toml` from its
  working folder and every folder above it, so neither a project nor the repository
  around it can change how the bot is built. Only Linux on the
  host's architecture is supported until the Windows phase. gcc is found on `PATH` as `cc`
  or `gcc`. The install command is `pkexec` + dnf/apt-get/pacman/zypper, which the app
  runs only after the user agrees.


### C-006 — Keyring entries and backend
- **Date:** 2026-11-22
- **Choice:** keyring 4's default `v1` feature, which on Linux uses the Secret Service
  through zbus in pure Rust (no libdbus), so KDE Wallet or GNOME Keyring. Every entry uses
  the service `io.github.kotoanidev.RustBotCreator` (D-061). A bot token's user is
  `<project id>/discord-token` (D-050). Whitespace around a pasted token is trimmed. The
  round-trip test touches the real keyring, so it is `#[ignore]`d (CI containers have no
  Secret Service).

### C-007 — How the runner runs a bot
- **Date:** 2026-11-22
- **Choice:** The bot starts in its own folder (`bots/<project id>/`), so a `.env` in the
  project is not read when the app runs it. The token and the test server go only into
  the child's environment. Without a test server, `DISCORD_TEST_GUILD_ID` is removed, so
  an inherited one does not apply. stdin is null. stdout and stderr are read on two
  threads and sent line by line (lossy UTF-8) to a `Sender<LogLine>` the caller owns.
  Stopping sends SIGINT, which the generated bot handles like Ctrl+C, through the `kill`
  program (std cannot send it and unsafe code is denied), then kills the bot after a grace
  period. Dropping a `Bot` stops it with 5 s of grace. The test server (D-065) is a
  one-line file `bots/<project id>/test-server`, checked to be a Discord id.

### C-008 — The app shell of the slice
- **Date:** 2026-11-22
- **Choice:** The UI language follows `LC_ALL`, `LC_MESSAGES` or `LANG`: Polish for `pl*`,
  English otherwise, with a missing Polish message falling back to English. The Fluent
  files are `crates/rustbot-creator/locales/{en,pl}/main.ftl`, compiled into the app, and a
  test checks that both define the same messages and that every `t("…")` exists. There is
  no activity bar yet, because the slice has one activity. Panels cannot be closed until
  a menu can reopen them. "New project…" names the project after the folder picked in the
  system dialog and adds a `/ping` command (`templates/ping.json`).
  `rustbot-creator <folder>` opens a project directly. The default gpui-kit theme is used
  until Phase 5.

### C-009 — How the graph canvas works
- **Date:** 2026-11-22
- **Choice:** Nodes are 190 px wide, with a 30 px title tinted by category (trigger blue,
  action green, logic yellow, data magenta, an unknown type red) over 24 px rows of pins:
  inputs on the left, outputs on the right. Wires are cubic Béziers, coloured by pin kind
  (flow in the text colour, each data type its own colour). A left drag on empty canvas
  or the scroll wheel pans the view; there is no zoom yet. An input keeps one wire, so a
  new wire replaces the old one. A new node gets the next free `nN` id and empty fields,
  and appears with its first row under the pointer. A node without a layout position is
  drawn in a column on the left. Undo keeps up to 200 steps, and a drag is one step.
  Autosave waits 500 ms (D-068) and also runs when the app quits; a failed save goes to
  the logs and is retried on the next change. Node, pin and field names come from Fluent
  (`node-…`, `pin-…`, `prop-…`), and a plugin's node without them shows its id. The
  geometry and edits live in `src/canvas.rs` and are unit-tested without the UI.

### C-010 — `wgpu` 30.0.1 in the lockfile
- **Date:** 2026-11-22
- **Choice:** With gpui-kit's `test-support` (D-070), cargo also resolves the optional
  `gpui-fast` backend, which pins `wgpu = "=30.0.1"`, while gpui's own backend had locked
  30.0.2. `cargo update -p wgpu --precise 30.0.1` satisfies both. `gpui-fast` is in the
  lockfile but is never built (`cargo tree -e normal,dev` does not list it). Undo this
  when gpui-kit's two backends agree on wgpu again.

### C-011 — The properties form and the bot settings
- **Date:** 2026-11-22
- **Choice:** Each field of the selected node is a one-line input labelled from Fluent
  (`prop-<node>-<field>`). Template fields list the variables the graph's trigger
  provides, such as `{user.name}`. Typing writes through `AppState::edit`, and the first
  change after a field gains focus starts one undo step, so Ctrl+Z takes back a whole
  edit, not a letter. After an undo the inputs show the restored text. With no node
  selected, the panel shows the bot settings. The token input is masked, is saved to the
  keyring by Enter or "Save" on a background thread, and is cleared at once, so the token
  does not stay in the window. The panel only reports whether a token is stored and never
  shows it. The test server is saved on Enter or when the field loses focus, and a
  non-numeric id is reported under it. `shortcut:` multi-line text (a long reply) waits
  for gpui-kit's textarea, which has another state type.

### C-012 — How Run works in the app
- **Date:** 2026-11-22
- **Choice:** Run saves, refuses while any graph has problems, and then goes through
  these steps. Trust (D-064) is stored as one canonical path per line in
  `trusted-projects` in the data folder, so a copy or move asks again; a path with a line
  break is never stored. A missing gcc or private Rust is installed only after a dialog
  (gpui's `window.prompt`) says what happens and how much it downloads. Codegen runs on
  the UI thread (it takes milliseconds). Installs, builds and the keyring run on
  background threads, and their lines are moved into Logs every 50 ms. The token is read
  from the keyring just before the start and dropped right after. With no token, the log
  points to the bot settings. Stop moves the bot to a background thread and stops it with
  5 s of grace. Logs keep the last 5000 lines and follow new ones. Library errors keep
  their English text inside a translated sentence.

### C-013 — Plain logs and clickable links
- **Date:** 2026-11-23
- **Choice:** the runner sets `NO_COLOR=1` for the bot, because tracing writes colour
  codes even into a pipe, and the Logs panel shows plain text. A log line that is only an
  `https://` address is drawn as gpui-component's `Link`, which opens it in the browser.
  This covers the invite link of D-072. Links inside longer lines stay plain text.

### C-014 — How the console step works (D-075)
- **Date:** 2026-11-23
- **Choice:** each log line is its own gpui-base `SelectableText`, in document order. The
  window's selection spans them, and `Root` copies the selection on Ctrl+C. One shared
  handle for all lines keeps only the last line in this gpui-base version.
  - **Focus:** the panel takes focus on click, so Ctrl+C reaches the window.
  - **Lines:** each line knows its source (app, build, bot) and its level. The level is
    read from cargo's `error:`/`warning:` and tracing's `ERROR`/`WARN`; any other bot
    stderr counts as an error.
  - **Colours:** errors are red, warnings yellow, build output muted, and the app's own
    step messages semibold.
  - **Clear:** "Clear" sits in the panel's tab-bar toolbar, like VS Code's panel actions.

### C-015 — How command options are stored and generated (D-076)
- **Date:** 2026-11-23
- **Choice:**
  - **Storage:** the options are the slash command's `options` prop: a JSON list of
    `{ "name", "type", "description", "required" }`. The project schema version does not
    change, since props were already free JSON.
  - **Types:** text, number (Discord's NUMBER, a decimal), yes/no, user, channel and role.
    Role is a new data type with `id`, `name` and `mention`.
  - **Validation:** Discord's rules, checked as problems. A name is 1-32 lower-case
    letters (or letters without case), digits, `-` or `_`, and is unique. A description is
    1-100 characters. Required options come first, and there are at most 25.
  - **Variables:** an option is `{option.<name>}`. Users, channels and roles need a
    field, as `{option.target.mention}` does.
  - **Generated code:** the locals are named by position (`option_0_name`), since option
    names may have letters that are not ASCII. An optional option that is not filled in
    is empty text, and yes/no prints `true`/`false`.
  - **Editor:** the Properties panel shows each option as a box with name, a type
    drop-down, description, "Required" and "Remove", and "+ Add option" below. A new
    option is text, named `option1`/`opcja1` (the first free one), with an empty
    description that Problems asks for. Text edits are one undo step per focus. Adding,
    removing, the type and "Required" are one step each.

### C-016 — Generated messages notify only what their author wrote
- **Date:** 2026-11-23
- **Choice:** a security review found that text from users reached replies unfiltered:
  `/ban reason:@everyone`, or a nickname, could ping the whole server. Every generated
  reply now sets Discord's `allowed_mentions`:
  - a `{user.mention}` or `{option.<user or role>.mention}` variable notifies that user
    or role;
  - `@everyone`/`@here` notify only when typed into the template itself;
  - anything else notifies no one, including `<@id>` typed by hand.

  Discord allows `@everyone` and `@here` together or not at all. So when a template
  allows them, every variable's value except the `….mention` ones goes through a generated
  `quiet()`. It puts a zero-width space after every `@` and one in front of the value. A
  user's "@everyone" cannot ride on an author's "@here", and neither can a value glued to
  an `@` in the template, as in `@{option.x}` with "everyone".
  This is the safe default. The owner may want a per-node switch later.

### C-017 — Typed fields, choices and how conditions are generated
- **Date:** 2026-11-23
- **Choice:**
  - **Field kinds:** `PropKind::Variable(type)` holds a variable's name (`user`,
    `option.target`). A role field also takes an ID of 17-20 digits that fits a Discord ID
    (D-080). `PropKind::Choice(list)` holds one value of a fixed list.
  - **Permission list:** "Has permission?" offers the 13 permissions bots check most, by
    lower-case name. Administrators pass every check, since Discord gives them every
    permission.
  - **Defaults:** a new node starts with empty fields, and Problems asks for the required
    ones. No user is chosen for it, so a later "Ban" never targets the author by default.
    `add_node` now gives the options list `[]` rather than `""`.
  - **Form:** fields appear in the node type's order. Pickers are drop-downs, and a role
    picker shows an ID field after "Role with ID…". A picker's list is rebuilt when the
    variables change, such as after adding an option.
  - **Generated code:** a condition becomes `if … { yes } else { no }`, and an empty branch
    is left out. A node that two branches reach is generated in each of them, and only a
    real loop is an error. The command module allows clippy's nested-`if` lints, since the
    nesting mirrors the graph.
  - **Compare:** "equals", "does not equal" and "contains" lower-case both sides (D-081).
    "Greater" and "less" parse both sides as numbers and are false if either is not one.

### C-018 — How moderation actions are generated
- **Date:** 2026-11-23
- **Choice:**
  - **Number fields:** `PropKind::Number { max }` takes a number above 0 and at most
    `max`, or a `{variable}` template. Problems checks a typed number (`BadNumber`); a
    template's number is checked when the bot runs. "Minutes" uses `max` = 40 320.
  - **Fields:** each action takes the user (a user variable), its own fields, and an
    optional "Reason (audit log)" with templates, left out when empty. "Delete their
    messages" starts at "don't delete" and maps to `delete_message_seconds`.
  - **"error" wired:** the action runs in an `async` block. A failure prints
    `WARN /<command>: <error>` (a warning in Logs) and runs the "error" branch.
  - **"error" not wired:** `?` ends the command handler, and djs-rs logs the error.
  - **What counts as a failure:** Discord refusing the call, a command from a DM (no
    server), an option that was not given, or minutes out of range.
  - **Permissions:** an action does not check the author's permissions itself. That is
    "Has permission?" in the graph (D-014's example), and D-085 gates and warns.

### C-019 — Which actions D-085 gates and warns about
- **Date:** 2026-11-23
- **Choice:**
  - **Permission field:** `NodeType` has an optional `permission`, which plugins can set
    too: `ban_members` for "Ban", `kick_members` for "Kick", `moderate_members` for "Time
    out" and `manage_roles` for the role actions.
  - **Acting on the author:** an action that targets the command author is not gated
    and gets no warning. That way "/verify" can give its user a fixed role, and members
    can still see it. A role picked from an option is gated even then, since the author
    could pick any role.
  - **The gate:** a command's `default_member_permissions` holds every permission its
    gated actions need. Discord then asks for all of them.
  - **Warnings:** warnings are a separate list (`Registry::warnings`). Problems shows
    them in yellow after the problems, and Run ignores them. An action warns if any path
    from the trigger reaches it without passing the "yes" of a "Has permission?" for
    `user`, the author. A check on the target guards nothing.

### C-020 — How messages are built
- **Date:** 2026-11-23
- **Choice:**
  - **Field kinds:** `PropKind::Bool` is a checkbox, and a new node starts with `false`.
    `PropKind::Color` takes `#RRGGBB` or a template, and Problems flags any other typed
    value (`BadColor`).
  - **Message fields:** "Reply" and "Send to a channel" share their fields: text, then
    the node's own fields, then `embed_title`, `embed_description`, `embed_color`,
    `embed_image` and `embed_footer`. The form folds the `embed_…` fields under "Embed",
    which opens by itself when one of them is filled in. A message with no text, title
    or description is `EmptyMessage`.
  - **Embed values:** a field whose variables come out empty when the bot runs is left
    out, and so is a colour from a variable that is not one. Discord's embed limits are
    checked by djs-rs, and a failure ends the command with an error in Logs.
  - **Several replies:** with more than one "Reply" in a command, each checks a
    `replied` cell. The first replies, and the later ones send follow-ups, so a second
    "Reply" never fails.
  - **"Send to a channel" has no "error" output** (D-082 covers moderation). A failed
    send ends the command with an error in Logs. Mentions follow C-016, as in replies.
  - **Not replying (D-089):** a command warns unless every way through it replies. Nodes
    wired to one pin run in turn, so one that always replies is enough. A node's output
    pins are branches, and each one must reply. An unwired "error" does not count, since
    the command fails there anyway.

### C-021 — How problems show on the graph
- **Date:** 2026-11-23
- **Choice:**
  - **Border colour:** red for a node with a problem, yellow for a warning, and a problem
    wins. A selected node keeps that colour with a thicker border; otherwise selection
    keeps its usual ring colour.
  - **Clicking a problem:** a click on a line in Problems opens its graph, selects its
    node, and centres the node in the editor, waiting for the canvas's size if needed. A
    problem of the whole graph, such as "no trigger", only opens the graph.
  - **Cost:** the editor checks the open graph as it draws (0.7 ms for 500 nodes, frames
    within D-038, `docs/PERFORMANCE.md`). Keeping the result per change is the noted
    upgrade.

### C-022 — How event graphs work
- **Date:** 2026-11-23
- **Choice:**
  - **Triggers:** `core.member_join`, `core.member_leave`, `core.message`, `core.ready`,
    `core.reaction_add` and `core.reaction_remove`.
  - **What each provides:** `user` and `server` for members; `user`, `channel`,
    `server` and `message` (its text) for messages; the same with `emoji` instead of
    `message` for reactions; nothing for "ready".
  - **Intents:** members need `GUILD_MEMBERS`, messages need `GUILD_MESSAGES` and
    `MESSAGE_CONTENT`, and reactions need `GUILD_MESSAGE_REACTIONS`.
  - **Where nodes work:** `NodeType` has an optional `triggers` list, which plugins can
    set too. "Reply" works under commands and "New message", "Has permission?" only
    under commands (Discord sends a member's permissions with commands only), and
    "Delete message" only under "New message". Elsewhere it is `WrongTrigger`. "Only the
    command author sees it" outside a command is `PrivateOutsideCommand`.
  - **The palette** offers no triggers, since a graph has its own, and only the nodes
    that work under the open graph's trigger.
  - **Missing data:** "Has role?" says no where the event carries no roles (a member who
    left, a removed reaction). A reaction's user name is fetched, since the event carries
    only the ID.
  - **Generated code:** each event graph is `generated/src/events/<name>.rs`, registered
    with `client.on(Events::…)`. "Ready" adds a handler next to the bot's own. A failed
    moderation action logs `WARN <graph>: <error>`.
  - **Bots:** with "Also messages from bots" on, the bot still skips itself by comparing
    the author with its application ID, which is the bot's user ID.

### C-023 — Adding and deleting graphs in the project tree
- **Date:** 2026-11-23
- **Choice:**
  - "+" by "Commands" opens a name field above the tree. Enter adds the command if the
    name follows Discord's rule (1–32 lower-case letters, digits, `-`, `_`) and is free;
    otherwise the field says why. Leaving an empty field closes it. The new graph holds
    the "Slash command" trigger with that name.
  - "+" by "Events" lists the event triggers. The graph is named after the event
    (`member_join`), then `member_join_2` and so on, since one event may have several
    graphs.
  - New graphs are saved at once and opened.
  - Right-click on a graph → "Delete" asks first, in a system dialog. Pending changes are
    saved, both files go, and the undo steps of the other graphs are kept.
  - A bot refused for privileged intents (close code 4014) prints a plain line and the
    portal link on its own line, which the Logs open (D-095, as in D-072).

### C-024 — How buttons and menus work
- **Date:** 2026-11-23
- **Choice:**
  - **Storage:** a message node holds `buttons` (`{id, label, style, url}`, style
    `primary`, `secondary`, `success`, `danger` or `link`) and `menu` (`{id, label}`), plus
    `menu_placeholder`, `wait` (minutes, up to a day), `only_author` and `not_yours`.
  - **Outputs:** ids are `b1`, `b2`, … and `o1`, `o2`, …; each id is also its output pin, so
    a wire stays on its button when others are removed. `Registry::pins(node)` adds them to
    the type's pins, with "timeout" when the message waits. Removing a button in the form
    removes its wires in the same undo step.
  - **Rules:** at most 5 buttons (label 1–80 characters) and 25 options (1–100); a link
    needs an `http(s)://` address.
  - **After a click that always works:** the handler is a new run. `user` is the clicker,
    `channel` and `server` are the click's, and nodes behave as under a command:
    "Reply" answers the click, and "Has permission?" checks the clicker. Validation
    tracks this per node; "Has permission?" lists `core.click` among its triggers.
  - **After a click waited for:** every variable keeps its value, "Reply" answers the
    click, and "then" follows when the wait is over. The buttons are removed from the
    message when the wait ends, except on a private reply, which cannot be edited that way.
  - **Answering clicks:** a click that no "Reply" answers is acknowledged silently, so
    Discord does not say it failed.
  - **Custom IDs:** `c.<graph>.<node>:<item>` for commands and `e.…` for events, with
    `:<author ID>` when only the author may click; a waited-for click uses `wait:<item>`.
    The menu's item is `menu`, and the option's id is its value.
  - **Only the author:** ticking it fills "Answer to others" with a translated default;
    an empty answer acknowledges the click without a message. "Bot is ready" has no user,
    so it is a problem there.
  - **Form:** the "Buttons and menu" section folds like "Embed". The menu hint shows once
    the menu has options, and the answer to others once "only the author" is ticked.
  - **Palette:** in a graph with buttons or a menu, it also offers nodes that work after
    a click, such as "Reply" in an event graph.
  - **Permissions after a click (security review):** Discord's gate (D-085) covers only
    using the command, not clicking. A click that always works starts unchecked, and
    "Has permission?" after it checks the clicker. After a click waited for that anyone
    may make, a check of `user` checks who started it, not the clicker, so an action that
    needs a permission gets the warning "anyone who clicks can make it run" until "only
    the user who started it" is ticked.

### C-025 — How the actions of D-102 work
- **Date:** 2026-11-23
- **Choice:**
  - **Failures:** "Send a DM", "Add a reaction", "Pin", "Unpin", "Edit message" and
    "Change nickname" have an "error" output, as moderation has (D-082). A DM fails when
    the user takes none from the server; an unwired "error" ends the run with a line in
    Logs.
  - **Which message:** "Last sent by the bot" is the last message any "Reply", "Send to a
    channel" or "Send a DM" sent in this run, kept only when some node uses it. It costs
    one more request per reply, since Discord does not return the reply. Before any
    message was sent, it is an error. "The event's message" is the problem
    `NoEventMessage` outside "New message" and reactions, and after a click that always
    works. Private replies cannot be pinned, reacted to or edited.
  - **Emoji:** a Unicode emoji as typed, or a custom one as Discord writes it,
    `<:name:id>` or `<a:name:id>`; `{variables}` are allowed, e.g. `{emoji}`.
  - **Edit message:** new text and embed; an empty text keeps the old one, and buttons
    stay.
  - **Change nickname:** an empty nickname, or a template that comes out empty, gives the
    user back their own name. It needs "Manage nicknames" on others (D-085).
  - **Wait:** up to 900 seconds (15 minutes), as long as a command can still follow up;
    a `{variable}` is checked when the bot runs.

### C-026 — How the bot's memory works
- **Date:** 2026-11-23
- **Choice:**
  - **Where:** `data.json` in the folder the bot runs in. For the app, that is the bot's
    folder in the app's data (`bots/<project id>/`, beside `test-server`), so a
    `cargo clean` never wipes it. A bot built by hand keeps it in its working folder.
  - **Layout:** `{ "<variable>": { "<key>": value } }`. The key is empty for a global
    variable, a server's ID, a user's ID, or `server:user` for a user on a server. Whole
    numbers are kept without `.0`.
  - **Safety:** each change writes a new `data.json.tmp`, syncs it and renames it over
    `data.json`, so a crash leaves one whole file. If saving fails, the old value stays
    and the node's "error" output runs. A damaged `data.json` stops the bot at start, so
    it is never overwritten. Changes take a lock, so two "+1"s at once both count.
  - **Reading:** `{memory.<name>}` is the value of whoever and wherever this run is.
    `{memory.<name>.<user variable>}` (e.g. `{memory.warnings.option.target}`) is another
    user's. Values are read where they are used, so a message after "Change by" shows the
    new value.
  - **Nodes:** "Set", "Change by" and "Reset" have "Whose": empty means the user who
    started the run. They also have an "error" output. "Change by" on a text variable,
    or a typed value that is not a number for a number, is a problem. A `{variable}` is
    checked when the bot runs. A variable kept per server or per user where none is known
    (e.g. in "Bot is ready") is a problem.
  - **Variables (as D-109 changed it):** "New variable…" makes a variable kept per user
    on each server (warnings, XP). It holds a number, or text when "Set" already has text
    typed. Its name follows the rules for a command's name. A number variable refuses a
    starting value that is not a number. Renaming fixes the "Variable" fields and
    `{memory.…}` in every graph, but undo steps taken before keep the old name. Deleting
    asks first, and the saved values stay in `data.json`. Memory changes have no undo
    yet.

### C-027 — How forms work
- **Date:** 2026-11-23
- **Choice:**
  - **Order:** Discord takes a form only as the first answer to a command or click, within
    3 seconds. So before "Show form" only conditions may run, and it must be alone on its
    wire: nodes on one output run one after another. A sent form cannot be answered with
    another form.
  - **Waiting:** an empty "wait" waits 15 minutes. The form's custom ID is
    `form:<node>`, and only the user who opened it can send it. After "time ran out",
    nothing can reply, because the form was the answer: "Reply" there is the problem
    `ReplyAfterClosedForm`. "Send to a channel" and DMs still work.
  - **Answering:** after a form is sent, "Reply" answers it. If nothing does, a form
    opened from a click is acknowledged quietly. A form opened from a command cannot be,
    so a private "thinking" answer is made and deleted at once; otherwise Discord shows
    an error in the form.
  - **Fields:** `{field.<name>}` is known after "then", for the rest of the run, until a
    click that always works starts a handler of its own. An optional field left empty is
    empty text. `-` and `_` in a name make the same Rust name, so two names that differ
    only there are the problem `BadForm`. The title, labels and hints are plain text,
    without variables.

### C-028 — How bot templates are made
- **Date:** 2026-11-23
- **Choice:**
  - **Files:** each template is a folder in `crates/rustbot-creator/templates/<id>/`,
    with one graph JSON per command or event, built into the app; nothing is downloaded
    (D-060). The `/ping` of D-057 moved to `templates/empty/`.
  - **Texts:** a string that is exactly `"$tpl-…"` is the Fluent message of that name,
    in the app's language (D-118). Braces in those messages are written `{"{"}` and
    `{"}"}`, as Fluent wants.
  - **Layout:** nodes are placed when the project is made: columns by their distance
    from the trigger, rows in the order the wires reach them. Templates have no layout
    files.
  - **Contents:**
    - "Moderation" keeps warnings in a number variable per user on each server.
    - "Roles from a menu" toggles each role with "Has role?". Its role changes are
      warnings (C-024), because anyone may pick an option: that is the point of a role
      menu.
    - "Welcome" needs the Server Members intent, switched on in the portal (D-095).
  - **Checks:** a test makes every template in the current language. Its only problems
    must be empty channel and role fields, and with those filled in it must generate.
    `template_bots_compile` builds each bot by hand.

### C-029 — How the rest of the catalog works
- **Date:** 2026-11-23
- **Choice:**
  - **Created things (D-120):** after "then", `created_channel`, `created_thread` and
    `created_role` are variables with `id`, `name` and `mention`. They are known on
    "then" only, like a form's fields, and a second node of the same kind replaces the
    first for the rest of the run.
  - **Private channels (D-121):** @everyone is denied "View channel". The chosen user,
    the chosen role and the bot are allowed to view, send and read the history. The bot
    finds its own ID with one "Get Current User" request.
  - **Threads:** public, from the last message the bot sent, the event's message, or no
    message in a channel. Members may start threads by default, so no permission is
    checked.
  - **Permissions:** the channel nodes need "Manage channels" and the role nodes "Manage
    roles", with the usual warning when nothing checks them (D-085).
  - **Ranking (D-122):** a "Data" node. A variable per user ranks every user, and one per
    user on each server ranks this server. An empty count lists 10, and nobody ranked
    gives "—". Its mentions notify nobody.
  - **"Levels" template:** 1 XP per message from members, not bots, kept per user on
    each server. Since "New message" asks for the Message Content intent, the bot needs
    it switched on in the portal (D-095).
  - **A fix found on the way:** an action whose "error" is wired ran in an `async` block
    that borrowed the "last sent message" cell. That made the handler not `Send`, so a
    bot pinning "the last one" with "error" wired did not build. The block now reads a
    copy.

### C-030 — Which catalog nodes need a permission check
- **Date:** 2026-11-23
- **Choice:** a security review found two holes in C-029, fixed in the same day's
  commit:
  - "Create channel" took the moderation rule of D-085, "acting on the author is
    allowed", from its "user" field. That field only says who sees the channel, so any
    member could make the bot create public channels. Now a channel needs "Manage
    channels", with the warning and Discord's gate, unless it is a ticket: private, for
    the author, and seen by a fixed role or none.
  - "Add to thread" needed nothing, and the bot may see private threads its users cannot.
    It now needs "Manage threads", which "Has permission?" lists too. A thread the bot
    just created is public, so adding to it needs nothing.
  - Members can still open as many tickets as they like. Limiting that, e.g. one ticket
    per member through memory, is up to the bot's author.
---

## Open questions (to ask the owner when relevant)


- **Polish for gpui-kit's own strings (D-037):** gpui-component compiles its rust-i18n
  YAML into itself, so the app cannot add a Polish one at runtime. A few component
  strings, such as an input's context menu or "Search...", stay English. Contribute a
  Polish translation upstream to gpui-component, or ask gpui-kit for a runtime hook?

The agenda for detailed planning. Each one becomes a `D-NNN` once the owner answers it.

- **Project format details** (D-019, D-040): the JSON schema of graphs, and migrations
  between schema versions.
- **Plugins** (D-016): what they may extend, the capability list, plugin API versioning and
  stability, distribution (a registry?), and signing.
- **Themes:** importing VS Code themes.
- **Toolchain details** (D-017, D-039): CMake for `voice`, toolchain updates, and build
  caching to make the first build faster.
- **Other hosts** besides Cloudflare (D-013).
- **GitHub integration** (D-021): the scope of features (create a repository, push, pull,
  releases, Actions) and the OAuth scopes they need.
- **Community files:** CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, issue templates, and when
  to add them.
- **RustBot Cloud** (D-125), before its pilot:
  - where the service code lives (a separate private repository?) and under which
    licence;
  - what it runs on: the company's own servers, or Cloudflare Containers (D-013 stays the
    "bring your own Cloudflare account" path);
  - payments, and VAT invoicing.
- **Paid templates and a marketplace:** templates are compiled into the app (C-028). Paid
  ones need a package format installed later, tied to plugin distribution and signing.
  Generated code stays MIT-0 (D-023), even from a paid template.
- **Name and trademark:** "RustBot" contains "Rust". Check the Rust Foundation's trademark
  policy before the brand, domain and logo are fixed.
- **Templates for a paid shelf:** economy and collecting (gacha), tickets and verification.
  Build them now as free templates, or keep them for paid ones?
