# Logging Overview

## Events

- `app:booted`
- `db:init:ok` / `db:init:err`
- `db:migrate:ok` / `db:migrate:err`
- `db:initialized`
- `fs:read:ok` / `fs:read:err`
- `fs:write:ok` / `fs:write:err`
- `project:scan:ok` / `project:scan:err`
- `git:ok` / `git:exit`
- `run:started`
- `run:exit:ok` / `run:exit:err`
- `run:pty:error`
- `editor:save:ok` / `editor:save:err`
- `ui:run:clicked` / `ui:run:err`
- `ui:stop:clicked` / `ui:stop:err`
- `ui:window:error`
- `ui:promise:unhandled`

## Levels

- `info` indicates successful operations and normal state changes.
- `error` captures failures and unexpected states.
- `debug` is emitted only in development builds for verbose diagnostics.

## Sinks

Logs are written to stdout and to the rotating log file inside the Tauri application log directory. Attach the console in development to mirror native logs into the browser devtools.
