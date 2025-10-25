#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod migrations;

use std::fs;

use tauri::{path::app_data_dir, AppHandle, State};

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

fn prepare_database(config: &tauri::Config) -> Result<String, String> {
    let data_dir =
        app_data_dir(config).ok_or_else(|| "app data directory unavailable".to_string())?;
    fs::create_dir_all(&data_dir).map_err(|err| err.to_string())?;

    let db_path = data_dir.join(DATABASE_FILE);
    let path_str = db_path
        .to_str()
        .ok_or_else(|| "database path contains invalid UTF-8".to_string())?;

    Ok(format!("sqlite:{path_str}"))
}

pub fn run() {
    let context = tauri::generate_context!();
    let db_url = prepare_database(context.config()).expect("failed to prepare database");

    tauri::Builder::default()
        .manage(DatabaseConfig {
            url: db_url.clone(),
        })
        .plugin(
            tauri_plugin_sql::Builder::new()
                .add_migrations(&db_url, migrations::definitions())
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_pty::init())
        .invoke_handler(tauri::generate_handler![ping, resolve_database_url])
        .run(context)
        .expect("error while running Projectlib");
}
