#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "desktop")]
mod git;
pub mod migrations;
#[cfg(feature = "desktop")]
mod runs;
pub mod terminal;

#[cfg(feature = "desktop")]
use std::{error::Error, fs, io};

#[cfg(feature = "desktop")]
use tauri::{AppHandle, Manager, State};

#[cfg(feature = "desktop")]
const DATABASE_FILE: &str = "projectlib.db";

#[cfg(feature = "desktop")]
struct DatabaseConfig {
    url: String,
}

#[cfg(feature = "desktop")]
#[tauri::command]
async fn ping(app: AppHandle, message: String) -> Result<String, String> {
    Ok(format!(
        "{message} :: reply from {}",
        app.package_info().name
    ))
}

#[cfg(feature = "desktop")]
#[tauri::command]
fn resolve_database_url(config: State<DatabaseConfig>) -> String {
    config.url.clone()
}

#[cfg(feature = "desktop")]
fn prepare_database(app: &AppHandle) -> Result<String, Box<dyn Error>> {
    let data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join(DATABASE_FILE);
    let path_str = db_path.into_os_string().into_string().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "database path contains invalid UTF-8",
        )
    })?;

    Ok(format!("sqlite:{path_str}"))
}

#[cfg(feature = "desktop")]
pub fn run() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();
            let db_url = prepare_database(&app_handle)?;

            app.manage(DatabaseConfig {
                url: db_url.clone(),
            });

            app.manage(git::service::GitService::new());

            let migrations: Vec<tauri_plugin_sql::Migration> = migrations::definitions()
                .into_iter()
                .map(Into::into)
                .collect();

            app_handle.plugin(
                tauri_plugin_sql::Builder::new()
                    .add_migrations(&db_url, migrations)
                    .build(),
            )?;

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_pty::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            resolve_database_url,
            git::operations::git_path_info,
            git::operations::git_set_path,
            git::operations::git_detect_repository,
            git::operations::git_status,
            git::operations::git_log,
            git::operations::git_branches,
            git::operations::git_switch_branch,
            git::operations::git_checkout,
            git::operations::git_stash_list,
            git::operations::git_stash_push,
            git::operations::git_stash_apply,
            git::operations::git_remote_list,
            git::operations::git_fetch_all,
            git::operations::git_pull,
            git::operations::git_push,
            runs::detect_project_runs,
            terminal::terminal_default_shell,
        ])
        .run(context)
        .expect("error while running Projectlib");
}

#[cfg(not(feature = "desktop"))]
pub fn run() {
    panic!("desktop feature is required to run the Tauri application");
}
