use tauri_plugin_sql::{Migration, MigrationKind};

pub fn definitions() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "create core projectlib tables",
        sql: r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            detected_lang TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            command TEXT NOT NULL,
            args TEXT,
            env TEXT,
            cwd TEXT,
            last_exit_code INTEGER,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS terminals (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            shell TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS git_remotes (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY DEFAULT 'global',
            theme TEXT,
            telemetry_enabled INTEGER,
            git_path TEXT
        );
        "#,
        kind: MigrationKind::Up,
    }]
}
