# RustBot Cloud API

The HTTP API between the app and RustBot Cloud (D-125, D-156–D-162). The app is its only
client; the service lives in a private repository (D-157). This file is the contract
both sides follow. Approved for the pilot (D-163); changes to it are decisions too.

## Basics

- Base URL: `https://<cloud host>/v1`. The host is fixed after the name and trademark
  check. Until then the app reads it from the `RUSTBOT_CLOUD_URL` environment variable,
  and Cloud stays hidden without it.
- JSON bodies, UTF-8. Every request sends `User-Agent: rustbot-creator/<version>`.
- Signed-in requests send `Authorization: Bearer <session>`.
- The app contacts Cloud only after the user starts it: sign-in, publishing, or opening
  a published bot's page. No background calls otherwise (D-060, D-125).
- Errors: an HTTP status and `{"error": "<code>", "message": "<English text>"}`. The app
  translates the codes it knows and shows the message for the rest.

| Code | Status | Meaning |
|---|---|---|
| `signed_out` | 401 | The session is missing, expired or revoked |
| `not_invited` | 403 | The pilot is by invitation (D-158), and this account is not invited |
| `limit` | 403 | The account's bot, size or memory limit is reached |
| `not_found` | 404 | No such bot on this account |
| `too_large` | 413 | The sources are over the size limit |
| `update_app` | 426 | This app version is too old for Cloud |
| `busy` | 409 | A deploy is already running for this bot |

## Signing in (D-160)

1. `POST /v1/logins` → `201 {"id": "<login id>", "url": "<browser URL>", "interval": 2,
   "expires_in": 600}`.
2. The app opens `url` in the browser. The user signs in with Discord there, and Cloud
   does the OAuth exchange with its own secret.
3. The app polls `GET /v1/logins/<id>` every `interval` seconds:
   - `202 {"status": "pending"}` — the user has not finished yet;
   - `200 {"session": "<session>", "user": {"id": "<Discord user id>", "name": "<name>"}}`
     — signed in, and the login is used up;
   - `403 not_invited`, or `410` when it expired.
4. The app keeps the session in the OS keyring (D-020).

`DELETE /v1/session` signs out and revokes the session. `GET /v1/me` → `{"id", "name",
"limits": {"bots", "memory_mb", "source_kb"}}`. The pilot's limits are 1 bot, 64 MB and
2048 KB (D-165).

## Bots

A bot is known by its project's id (D-050), so publishing the same project again updates
it.

- `GET /v1/bots` → `[{"project", "name", "state", "deployed_at", "codegen"}]`.
- `GET /v1/bots/<project>` → `{"project", "name", "state", "deployed_at", "codegen",
  "memory_kb", "restarts", "started_at", "has_token"}`.
  - `state`: `building`, `starting`, `running`, `stopped`, `failed` or `build_failed`.
  - `codegen`: the generated code's hash, so the app shows "out of date" as for local
    hosting (D-140).
- `PUT /v1/bots/<project>/token` `{"token": "<bot token>"}` → `204`. Sent only after the
  user's consent (D-161). Cloud stores it encrypted and never returns it.
  `DELETE /v1/bots/<project>/token` → `204`.
- `POST /v1/bots/<project>/deploys` → `202 {"deploy": "<id>"}`, with the body:

  ```json
  {
    "name": "<bot name>",
    "package": "<cargo package>",
    "codegen": "<hash>",
    "files": {"Cargo.toml": "<text>", "src/main.rs": "<text>", "...": "..."}
  }
  ```

  The files are the source export's (D-134), as UTF-8 text, with `/` in paths. No `..`,
  no absolute paths, no `target/` and no `.env`. `Cargo.toml` is required. Without a
  `Cargo.lock`, e.g. when the app never built the bot, Cloud resolves the versions
  itself. Cloud builds them in a sandbox (D-159, D-162), then restarts the bot on the
  new build. A failed build keeps the old one running.
- `GET /v1/bots/<project>/deploys/<id>` → `{"state": "queued" | "building" | "done" |
  "failed", "log": "<build output>"}`.
- `POST /v1/bots/<project>/start`, `POST /v1/bots/<project>/stop` → `204`.
- `DELETE /v1/bots/<project>` → `204`. It removes the bot, its token, its memory and its
  backups.

## Logs

`GET /v1/bots/<project>/logs?after=<cursor>` → `{"lines": [{"time": "<RFC 3339>",
"stream": "stdout" | "stderr", "text": "<line>"}], "cursor": "<cursor>"}`. Without
`after`, the last 200 lines. The app polls it every 2 s while the bot's page is open.
Cloud keeps 7 days of logs.

## Memory backups

The bot's memory is its `data.json` (D-143). Cloud copies it once a day and keeps 7
copies.

- `GET /v1/bots/<project>/backups` → `[{"id", "time", "size"}]`.
- `GET /v1/bots/<project>/backups/<id>` → the file.
- `POST /v1/bots/<project>/backups/<id>/restore` → `204`. The bot restarts with that
  memory.
