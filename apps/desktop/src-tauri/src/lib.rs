#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod git;
mod migrations;

use std::{error::Error, fs, io};

use tauri::{AppHandle, Manager, State};

const DATABASE_FILE: &str = "projectlib.db";

struct DatabaseConfig {
    url: String,
}

#[tauri::command]
async fn ping(app: AppHandle, message: String) -> Result<String, String> {
    Ok(format!(
        "{message} :: reply from {}",
        app.package_info().name
    ))
}

#[tauri::command]
fn resolve_database_url(config: State<DatabaseConfig>) -> String {
    config.url.clone()
}

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

            app_handle.plugin(
                tauri_plugin_sql::Builder::new()
                    .add_migrations(&db_url, migrations::definitions())
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
        ])
        .run(context)
        .expect("error while running Projectlib");
}
