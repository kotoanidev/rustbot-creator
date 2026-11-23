# Progress — resume point

> Updated at the end of every significant change. A fresh session must be able to continue
> from this file alone. Read `CLAUDE.md` and `docs/DECISIONS.md` first.

## Current phase
**Phase 0 — Initialization: ✅ done 2026-11-17.**
**Phase 1 — Detailed planning: ✅ done 2026-11-17** for everything Phase 2 needs (D-011 to
D-046). Later phases are planned in detail when each one starts (D-043).
**Phase 2 — Vertical slice: ✅ done 2026-11-23.** The owner ran `/ping` live on a test server.
**Phase 3 — Easy mode: ✅ done in code 2026-11-23; the owner's live test is pending.**
**Next: phase 5, first run + themes.** The phase order changed after phase 3 to follow the
incubation timeline (D-123–D-125): 5 → 6 → 7 → a public Linux release → Windows →
RustBot Cloud pilot → 4 → 8 → 9. The README describes the pre-alpha honestly now.

Phase 3 was planned 2026-11-23 (D-073–D-075). Step 1, the console (D-075, C-014), is ✅ done: lines select and copy with Ctrl+C, errors are red and warnings yellow, build output is muted, and "Clear" sits in the panel's toolbar. Step 1 (workspace skeleton + CI) is ✅ done.
The six crates of D-036 and the lints of D-046 are in place, and `cargo run -p
rustbot-creator` opens an empty "RustBot Creator" window (gpui-kit 0.10.0). The first CI
run was green: rustfmt 15 s, Ubuntu 280 s, Fedora 44 297 s.
Step 2 (project format v0, D-047–D-051, C-003) is ✅ done. `rustbot-core` has `Manifest`,
`Graph`, `Layout`, `GraphFile` and `Project::{create, open, save_manifest, save_graph}`,
with 12 tests: the round trip, byte-exact deterministic files, refusing to overwrite, the
schema check, unknown fields, graph names that would escape the folder, and symlinks
(refused on read and write, and a planted temporary file is never followed, from the
security review fix `f92e0e2`).

Step 3 (node registry, D-052–D-054) is ✅ done. `rustbot_core::registry` has `NodeType`,
`Pin`, `Prop`, `Variable` and `DataType`, all serde data for the future WASM plugins, plus
`Registry::{with_core, register, get, iter, validate}` and `Problem`/`ProblemKind` for the
"Problems" panel. `rustbot_core::template` parses `{path}` and `{{`/`}}`. The core nodes
are `core.slash_command` (out `then`; props `name` and `description`; provides `user`,
`channel` and `server`) and `core.reply` (in `run`, out `then`; prop `content` with
templates). 24 tests pass in `rustbot-core`.

Step 4 (code generation, D-056–D-059, C-004) is ✅ done. `rustbot_codegen::{generate,
write}` turn a validated project into `generated/` (`Cargo.toml`, `src/main.rs`,
`src/settings.rs`, `src/commands/{mod,<name>}.rs`), using tokens and prettyplease, with
snapshot tests (`insta`), injection tests, and the `generated_bot_compiles` test. That test
builds the bot against djs-rs 0.1.0-alpha.2 (D-069) with `clippy -D warnings` and runs as its own CI
job. 501 nodes generate in 7.6 ms (`docs/PERFORMANCE.md`). `rustbot-core` gained
`Project::{write_file, remove_file, files_under}`, which are symlink-safe.

Security review fix `fix(codegen)`: `write` owns all of `generated/` (it removes planted
`build.rs` and `.cargo/`), the bot's `Cargo.toml` sets `build = false` and turns off
`auto*`, and `generated/rust-toolchain.toml` pins Rust (D-062).

Step 5 (private toolchain, D-061–D-064, C-005) is ✅ done. `rustbot_toolchain::Toolchain`
has `locate`, `install`, `is_installed`, `build` and `target_dir`, plus `c_compiler()` and
`install_c_compiler_command()`. The real end-to-end test (`--ignored install`) installs
in 29 s, builds `/ping` in 42 s, and rebuilds in 1.1 s (`docs/PERFORMANCE.md`). Project
trust (D-064) is decided and belongs to the app shell step.

Step 6 (bot token in the keyring, C-006) is ✅ done. `rustbot_secrets::{bot_token,
set_bot_token, delete_bot_token}` are keyed by the project id, and the round trip passes
against KDE Wallet. Fix `4c7b764`: cargo runs in the cache folder with `--manifest-path`,
so `.cargo/config.toml` in or above a project is never read (security review).

Step 7 (runner, D-065, C-007) is ✅ done. `rustbot_runner::Bot::{start, exit_status,
stop}` runs a built bot with the token and test server in its environment, streams its
output as `LogLine`s, and stops it with SIGINT, then a kill after a grace period.
`rustbot_runner::{test_server, set_test_server}` keep the test server per user.
`Toolchain::bot_dir` is the project's folder in the app's data. The runner has not been
tried on a real built bot with a real token yet: that is the live test of step 9.

Step 8 (app shell, D-066–D-068, C-008, C-009) is in progress.
- 8a ✅ The window has a welcome screen (New/Open project through the system dialogs) and
  the dock: the project tree and Properties on the left, the editor in the centre, Logs
  and Problems below, and a status bar. All text is in Fluent, English and Polish.
- 8b ✅ The graph canvas (`src/panels/editor.rs`, `src/canvas.rs`) draws nodes, pins and
  wires. You can drag nodes, select one, wire pins by dragging, delete with Delete or
  Backspace, add nodes from a searchable list (right-click, "+", or a wire dropped on
  empty canvas, D-067), pan, and undo and redo with Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y.
  Changes are autosaved (D-068). Seen on screen; dragging and autosave were tried in the
  real window. Headless UI tests (D-070) drag a node, drop a wire to add a wired node,
  delete and then undo and redo, and search the right-click list. They found that a
  right-click took the focus back from the list's search, so the list now opens on
  release.

- 8c ✅ Properties (C-011): the selected node's fields, with translated labels and the
  template variables, edited with undo. With nothing selected, the bot settings: the
  masked token saved to the keyring and the test server. A headless test checks that
  typing edits the node and that undo restores the field. The form was seen on screen.

- 8d ✅ Run/Stop in the status bar (D-071, C-012), in `src/run.rs` and `src/trust.rs`:
  save → problems → trust (D-064) → gcc and Rust with consent → codegen → build →
  token and test server → start, with every line in Logs; Stop stops the bot. A
  headless test clicks Run on a project from elsewhere and checks that the trust
  question comes first and that Cancel builds nothing.

Step 8 (app shell) is ✅ done.

Step 9 (budgets) is measured, and every budget is met (`docs/PERFORMANCE.md`): warm
startup 150–280 ms, memory 108–130 MB, frame time with 500 nodes p99 6.8–10 ms, codegen
7.6 ms, rebuild 1.1 s. The canvas draws only what is on screen, with one path per wire
colour, and skips drawing in the first frame. Only a cold first start after a build is
slower (1.09 s).

The owner's first live run (2026-11-23) worked up to Run, then failed with 403. Discord
refuses to register commands in a server the bot is not on (`50001 Missing Access`).
The fixes:
- The generated bot checks the Ready server list (D-072). When the test server is
  missing, it prints a plain message and an invite link on its own line. The Logs panel
  opens such a line in the browser.
- The runner sets `NO_COLOR=1`, so tracing's colour codes no longer clutter the Logs.

After these fixes the owner's live test passed (2026-11-23): Run built the bot, and
`/ping` answered on the test server. Step 9 and Phase 2 are ✅ done.

The owner finds the console hard to read and wants it improved later. Colour codes are
gone, but selecting and copying text, levels and wrapping are still open.

## Exact next step
**Phase 3: the owner's live test of `/ban` and the events, then the next catalog part.**
Step 2 (the `/ban` example of D-014, D-073) is ✅ done in code:
- 2a, the command options (D-076);
- 2b, the variable picker: {x} and typed pickers, with "Role with ID…" (D-078, D-080);
- 2c, the conditions "Has permission?", "Has role?" and "Compare" (D-077, D-081), with
  branching codegen (C-017);
- 2d, the moderation actions "Ban", "Kick", "Time out", "Add role" and "Remove role", with
  "then" and "error" outputs (D-082–D-084, C-018). Moderation commands are gated by
  Discord's default member permissions, and Problems warns in yellow when an action is
  not behind "Has permission?" (D-085, C-019);
- 2e, the messages: "Reply" with "Only the command author sees it" (D-086), "Send to a
  channel" with "Channel with ID…" (D-088), the folding "Embed" section (D-079, D-087),
  follow-ups for later replies, and a warning when a command may not reply (D-089, C-020).

Step 3, problems on the graph (D-090, C-021), is ✅ done. Nodes with a problem have a
red border, and those with a warning a yellow one. Clicking a line in Problems opens the
graph, selects the node and centres it.

Step 4, the first part of the node catalog, the events (D-091–D-095, C-022, C-023), is ✅
done in code:
- six event triggers ("Member joined", "Member left", "New message", "Bot is ready",
  "Reaction added", "Reaction removed"), each a graph in `graphs/events/` and a module in
  `generated/src/events/`, with their variables and intents;
- "New message" skips bots unless "Also messages from bots" is on, and never answers
  itself; "Reply" answers the message; a new "Delete message" node;
- nodes that do not work under a trigger are a problem ("Reply" outside commands and
  messages, "Has permission?" outside commands), and the palette offers only those that
  work, without triggers;
- forms of events that need privileged intents explain how to switch them on, and a bot
  Discord refuses for them says so in Logs;
- "+" by Commands and Events in the project tree adds a graph; right-click → "Delete"
  removes one after asking.

Step 5, buttons and select menus (D-096–D-100, C-024), is ✅ done in code:
- "Reply" and "Send to a channel" have a folding "Buttons and menu" section: up to 5
  buttons (a colour, or a link) and a menu of up to 25 options. Each button and option
  is an output of the node, drawn on the canvas with its label.
- Clicks always work by default, through djs-rs component routes, also after a restart;
  after such a click, `user` is the clicker. With "Wait for a click (minutes)" the bot
  waits in the same handler, every variable stays, and "time ran out" follows if nobody
  clicks.
- "Only the user who started it can click" limits clicks, and others get a private
  answer.

A security review of step 5 found that the author's "Has permission?" was carried over
clicks, which Discord's gate does not cover. Fixed in `15faff4`: an action that needs a
permission after a click anyone may make is now a warning (C-024).

Step 6, more actions (D-101–D-103, C-025), is ✅ done in code: "Send a DM" (with the full
message form), "Add a reaction", "Pin", "Unpin", "Edit message", "Change nickname" and
"Wait". Message actions work on "Last sent by the bot" or "The event's message", and the
ones that can fail have an "error" output.

Step 7, the bot's memory (D-104–D-109, C-026), is ✅ done in code.
- The owner found the first design bad and unused: a "+" in the tree and a separate form.
  So variables are now made in the node (D-109). "New variable…" in a memory node's
  "Variable" list asks for a name.
- The variable's type (number or text), scope (global, server, user, user on a server)
  and starting value are edited under that list, for every node that uses it. They are
  kept in `project.toml`.
- The tree's "Memory" branch lists each variable with the nodes that use it; clicking one
  opens it, and right-clicking renames or deletes the variable.

The owner tested steps 2 and 4–6 live: everything works except typing a channel or role
ID, which closed the field after the first digit (fixed in `222a11e`). The nodes are "Set memory", "Change memory by" and "Reset memory",
and templates read `{memory.name}` or `{memory.name.option.target}`. The bot keeps
values in `data.json` in its folder. The `--ignored compiles` tests also build the
memory module and run it twice, checking that values last and that a damaged file is
never overwritten.

Step 8, modals (D-110–D-114, C-027), is ✅ done in code:
- "Show form" opens a form with 1–5 text fields, edited in its Properties. Each field has
  a name, a label, a hint, "long", "required" and lengths.
- It goes right after a command or click, with only conditions before it. Validation
  checks that.
- It waits up to 15 minutes. "then" goes on with `{field.<name>}`, and "Reply" answers
  the form. "time ran out" runs if nobody sent it.

Step 9, bot templates (D-115–D-118, C-028), is ✅ done in code. "New project" shows
cards, and the folder dialog follows:
- "Empty" (`/ping`);
- "Moderation with warnings" (`/ban`, `/kick`, `/timeout`, `/warn`);
- "Welcome" (join, leave, DM);
- "Roles from a menu" (`/roles`);
- "Reports by form" (`/report`).

Texts are in the app's language, and channels and roles are left empty for Problems to
point at. Building them found two generator bugs, fixed in the commit before.

Step 10, the rest of the catalog (D-119–D-122, C-029), is ✅ done in code, and with it
the node catalog of phase 3:
- "Create channel" (text, in a category, or private to a user, a role and the bot) and
  "Delete channel";
- "Create thread" (from the last or the event's message, or in a channel) and "Add to
  thread";
- "Create role" (name, colour) and "Delete role";
- "Ranking", whose `{ranking}` lists the top users of a number variable;
- the "Levels" template: XP for messages, `/level` and `/top`.

What a node creates is a variable after its "then": `{created_channel.mention}`, and
"Created channel" in the channel lists. Building the test bots found an older bug: an
action on "the last message" with "error" wired did not build. It is fixed (C-029).

A security review then tightened which of them need a permission check (C-030).

Every generated bot builds against djs-rs with `clippy -D warnings` (the `compiles`
test, and `template_bots_compile` for the templates). Memory, forms, templates and the
new nodes have not run live yet. Next:
Phase 5, step 1 (Settings and themes, D-127, D-130, C-031) is ✅ done in code:
- "⚙ Settings" in the status bar, or "Settings" on the welcome screen, has the language
  (it changes at once), the theme ("Follow the system", the built-in light and dark, and
  the user's own from `~/.config/rustbot-creator/themes`), the font size, and the state
  of the private Rust.
- Everything is kept in `~/.config/rustbot-creator/settings.toml`.
- The warm start is still about 130 ms.

1. The owner tries memory (e.g. `/warn` counting warnings with "Change memory by" and
   `{memory.warnings.option.target}`), forms, the templates (e.g. "Reports by form" with
   the channel filled in, or "Levels") and the new nodes (e.g. a private ticket channel)
   on the test server; fix what they find.
2. Measure the memory (RSS) of generated template bots (moderation, levels) connected to
   Discord, idle and on a few servers, and how many fit in 4 GB. Record it in
   `docs/PERFORMANCE.md`, and tell the owner if it is far from about 7 MB per idle bot.
   The owner starts the bot in the app; measuring its process needs no token.
3. Phase 5, step 2: the first-run wizard (D-128): language and theme, Rust and gcc with
   consent through an installer shared with Run and Settings, then a first project from
   a template. Then step 3: its Developer Portal guide with checks (D-129), using `ureq`
   from the workspace to ask Discord about the token.
4. The README still needs a screenshot labelled pre-alpha. Take it only of the app's own
   window: `spectacle -a` once caught another window when the app was not focused.

## Blocked on the owner
Nothing right now. The open questions in `docs/DECISIONS.md` belong to later phases.

## Blocked on djs-rs
Nothing. [djs-rs#3](https://github.com/kotoanidev/djs-rs/issues/3) (HTTP interactions +
wasm32) was fixed in djs-rs 0.1.0-alpha.2 (2026-11-22), which new projects use (D-069), so
the Cloudflare Workers phase is no longer blocked.
