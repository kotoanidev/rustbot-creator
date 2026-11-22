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
