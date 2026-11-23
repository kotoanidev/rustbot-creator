# RustBot Creator

**Build Discord bots on your desktop, with no coding or with full control over the code.**

RustBot Creator is a desktop application written in pure Rust for creating Discord bots on
top of [djs-rs](https://github.com/kotoanidev/djs-rs).

> **Pre-alpha.** It works, and you can build and run real bots with it, but it has not been
> released yet. There are no packages, and the interface will still change.

## What works today

- **Bots from templates or from scratch.** A new project starts from a template: empty,
  moderation with warnings, welcome, roles from a menu, reports by form, or levels.
- **A node-graph editor.** Each slash command and event is a graph of nodes, edited in a
  workspace in the style of VS Code, with a project tree, properties, problems and a
  console.
- **The nodes:**
  - triggers: slash commands with options, members joining and leaving, new messages,
    the bot starting, and reactions;
  - messages: replies (also ones only the author sees), messages to a channel, DMs,
    embeds, buttons, select menus and forms, plus editing, reacting and pinning;
  - moderation: ban, kick, timeout, roles and nicknames;
  - the server: creating and deleting channels and roles, and threads;
  - logic: permission and role checks, comparisons, and waiting;
  - memory: variables that last, kept for the bot, per server, per user or per member,
    and rankings.
- **Checks while you build.** Empty fields, wrong variables and missing permission checks
  are shown on the graph before the bot runs.
- **Build and run inside the app.** The graphs become readable Rust in the project's
  `generated/` folder. The first build installs a private Rust toolchain for your bots, in
  the app's data folder. The bot token is kept in the system keyring, and the bot runs with
  its logs in the app.
- **English and Polish.** Fully local: no account and no telemetry.

## Coming next

A first-run wizard that also guides you through the Discord Developer Portal, light and
dark themes, exporting the bot as source code or a ready build, hosting it on your PC, and
a public release for Fedora and Ubuntu, then Windows. After that come an advanced mode
with a code editor, and GitHub and Cloudflare integrations. The plan is in
[`ROADMAP.md`](ROADMAP.md).

## Building from source

On Fedora or Ubuntu, with Rust from [rustup](https://rustup.rs):

```bash
git clone https://github.com/kotoanidev/rustbot-creator
cd rustbot-creator
pkexec scripts/install-build-deps.sh   # once: the system libraries of the UI (dnf or apt)
cargo run --release -p rustbot-creator
```

To run a bot, you need a Discord application and its bot token from the
[Discord Developer Portal](https://discord.com/developers/applications).

## Status

Pre-alpha. Progress is in [`PROGRESS.md`](PROGRESS.md), and every decision with its
reasons is in [`docs/DECISIONS.md`](docs/DECISIONS.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above,
without any additional terms or conditions.

RustBot Creator is an independent project and is not affiliated with Discord Inc.
