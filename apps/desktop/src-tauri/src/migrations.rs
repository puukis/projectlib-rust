#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MigrationKind {
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MigrationDefinition {
    pub version: i64,
    pub description: &'static str,
    pub sql: &'static str,
    pub kind: MigrationKind,
}

pub fn definitions() -> Vec<MigrationDefinition> {
    vec![
        MigrationDefinition {
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
        },
        MigrationDefinition {
            version: 2,
            description: "add run status tracking",
            sql: r#"
        CREATE TABLE IF NOT EXISTS run_status (
            project_id TEXT PRIMARY KEY,
            status TEXT NOT NULL,
            last_run_id TEXT,
            last_command TEXT,
            last_args TEXT,
            last_env TEXT,
            last_cwd TEXT,
            last_exit_code INTEGER,
            started_at INTEGER,
            finished_at INTEGER,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        "#,
            kind: MigrationKind::Up,
        },
        MigrationDefinition {
            version: 3,
            description: "add editor state table",
            sql: r#"
        CREATE TABLE IF NOT EXISTS editor_state (
            id TEXT PRIMARY KEY DEFAULT 'global',
            open_files TEXT,
            active_file TEXT,
            view_state TEXT,
            updated_at INTEGER NOT NULL
        );
        "#,
            kind: MigrationKind::Up,
        },
    ]
}

#[cfg(feature = "desktop")]
impl From<MigrationDefinition> for tauri_plugin_sql::Migration {
    fn from(value: MigrationDefinition) -> Self {
        let kind = match value.kind {
            MigrationKind::Up => tauri_plugin_sql::MigrationKind::Up,
            MigrationKind::Down => tauri_plugin_sql::MigrationKind::Down,
        };

        tauri_plugin_sql::Migration {
            version: value.version,
            description: value.description,
            sql: value.sql,
            kind,
        }
    }
}
