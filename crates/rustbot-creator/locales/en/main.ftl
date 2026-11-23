## The welcome screen.
welcome-tagline = Build Discord bots on your desktop, with no coding or with full control over the code.
new-project = New project…
open-project = Open project…
pick-project-folder = Choose a bot project folder
error-open = Could not open the project: { $error }
error-create = Could not create the project: { $error }

## Panels.
panel-project = Project
panel-editor = Editor
panel-properties = Properties
panel-logs = Logs
panel-problems = Problems
tree-commands = Commands
tree-events = Events
properties-empty = Select a node on the canvas to edit it.
editor-empty = Open a command or event from the project tree.
logs-empty = The bot's output appears here when it runs.
logs-clear = Clear
problems-empty = No problems.

## The status bar.
status-stopped = Bot stopped
status-language = English

## Problems in a graph.
problem-no-trigger = The graph has no trigger, such as a slash command.
problem-several-triggers = The graph has more than one trigger.
problem-unknown-node-type = Unknown node type { $type }. Is a plugin missing?
problem-unknown-pin = The wire at { $pin } leads nowhere.
problem-wrong-direction = The wire at { $pin } goes the wrong way.
problem-kind-mismatch = { $from } and { $to } cannot be wired together.
problem-several-sources = { $pin } has more than one wire.
problem-missing-prop = The field “{ $prop }” is empty.
problem-unknown-prop = The field “{ $prop }” does not belong to this node.
problem-wrong-prop-type = The field “{ $prop }” has the wrong kind of value.
problem-template-unclosed = The field “{ $prop }” has a {"{"} that is never closed.
problem-template-stray = The field “{ $prop }” has a {"}"} with no {"{"}. Write {"}}"} for a brace.
problem-template-empty = The field “{ $prop }” has an empty variable, such as {"{}"}.
problem-unknown-variable = The field “{ $prop }” uses { $path }, which does not exist.

## The graph canvas.
canvas-add = Add a node
picker-search = Search nodes…
picker-empty = No node fits here.
error-save = Could not save { $file }: { $error }

## Built-in nodes, their pins and their fields.
node-core-slash_command = Slash command
node-core-reply = Reply
pin-then = then
pin-run = run
prop-core-slash_command-name = Command name
prop-core-slash_command-description = Description
prop-core-reply-content = Message

## The properties panel and the bot's settings.
props-variables = Variables: { $list }
bot-settings = Bot settings
bot-token = Bot token
bot-token-placeholder = Paste the token from the Discord Developer Portal
bot-token-checking = Checking the system keyring…
bot-token-saved = A token is saved in the system keyring.
bot-token-missing = No token is saved yet.
save = Save
bot-test-server = Test server ID
bot-test-server-hint = While you work, commands are registered on this server, where they appear at once.
bot-test-server-saved = Saved.
error-keyring = The system keyring cannot be used: { $error }
error-server-id = “{ $id }” is not a server ID: it has only digits.

## Run and Stop.
run = Run
stop = Stop
cancel = Cancel
status-running = Bot running
run-checking = Checking the project…
run-has-problems = The project has problems. Fix them first; they are listed under Problems.
run-cancelled = Cancelled.
trust-title = Build a project from elsewhere?
trust-detail = Building it runs code on this computer: the crates it depends on, and { $files } files of its own code in src/. Build it only if you trust where it came from.
trust-build = Trust and build
gcc-title = Install a C compiler?
gcc-detail = Bots are built with gcc, which this system does not have. Your system's package manager installs it and asks for your password.
gcc-install = Install gcc
gcc-installing = Installing gcc…
gcc-unknown-system = No supported package manager was found. Install gcc yourself, then press Run again.
gcc-failed = gcc could not be installed. The lines above say why.
rust-title = Install Rust?
rust-detail = Bots are built with Rust { $version }, which the app installs into its own folder: about 300 MB to download. Any Rust you have is not touched.
rust-install = Install Rust
rust-installing = Installing Rust…
run-generating = Generating the code…
run-building = Building the bot…
run-starting = Starting the bot…
run-started = The bot is running.
run-stopping = Stopping the bot…
run-exited = The bot stopped ({ $status }).
run-no-token = The bot has no token yet. Click an empty spot on the canvas and add it in the bot settings under Properties.
error-no-home = The home folder is unknown, so the app has nowhere to install Rust.
error-trust = The project could not be marked as trusted: { $error }
error-install = Rust could not be installed: { $error }
error-codegen = The code could not be generated: { $error }
error-build = The build failed: { $error }
error-start = The bot could not be started: { $error }
