# Database

Projectlib stores persistent state in a SQLite database that lives inside the
platform-specific application data directory (for example
`~/Library/Application Support/<app>` on macOS or
`%APPDATA%\<app>` on Windows). The database file is called
`projectlib.db` and is opened through the
[`tauri-plugin-sql`](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/sql)
plugin so both the Rust backend and the front-end can access it.

## Schema overview

The initial migration creates the following tables:

| Table | Purpose |
| --- | --- |
| `projects` | Workspace metadata such as id, human name, on-disk path, detected language, and timestamps. |
| `runs` | Saved command configurations keyed by project. Stores the command, serialized arguments, environment JSON, working directory, last exit code, and timestamp. |
| `terminals` | Tracks terminal sessions that belong to a project, including the shell and creation time. |
| `git_remotes` | Known Git remotes for a project (name/url pairs). |
| `settings` | Global application preferences such as theme, telemetry toggle, and Git executable path. |

### Table definitions

```
projects(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  path TEXT NOT NULL UNIQUE,
  detected_lang TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
)

runs(
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  command TEXT NOT NULL,
  args TEXT,
  env TEXT,
  cwd TEXT,
  last_exit_code INTEGER,
  updated_at INTEGER NOT NULL
)

terminals(
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  shell TEXT NOT NULL,
  created_at INTEGER NOT NULL
)

git_remotes(
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  url TEXT NOT NULL
)

settings(
  id TEXT PRIMARY KEY DEFAULT 'global',
  theme TEXT,
  telemetry_enabled INTEGER,
  git_path TEXT
)
```

`PRAGMA foreign_keys = ON;` is applied whenever a connection is opened so that
cascade rules are enforced.

## Migration workflow

Migrations are defined in `apps/desktop/src-tauri/src/migrations.rs`. Each
migration entry has an incrementing integer version, a short description, and
an SQL string to apply. The SQL plugin automatically runs any pending
migrations during application start-up before exposing the database to the
front-end.

To introduce a new migration:

1. Append a new `Migration` entry to `definitions()` with the next version number.
2. Describe the change and include the SQL that performs it (use
   `MigrationKind::Up` for forward migrations).
3. Update any TypeScript Zod schemas in `packages/db` to keep them aligned with
   the table changes.
4. Update this document if the schema meaning changes.

Because migrations run automatically on launch, no manual CLI steps are
required for developers beyond rebuilding the application.
