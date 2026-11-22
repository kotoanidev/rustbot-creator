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
