use notify::{event::EventKind, Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_fs::FsExt;

#[derive(Default)]
pub struct FsWatcherManager {
    watchers: Mutex<HashMap<String, RecommendedWatcher>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsChangeEvent {
    pub kind: String,
    pub path: String,
}

impl FsWatcherManager {
    pub fn new() -> Self {
        Self {
            watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn watch(&self, app: AppHandle, path: String) -> Result<(), String> {
        let canonical = Self::canonical_path(&path)?;
        let key = canonical.to_string_lossy().to_string();

        let mut watchers = self
            .watchers
            .lock()
            .map_err(|_| "failed to lock fs watcher map".to_string())?;

        if watchers.contains_key(&key) {
            return Ok(());
        }

        app.fs_scope()
            .allow_directory(&canonical, true)
            .map_err(|err| err.to_string())?;

        let app_handle = app.clone();
        let base_path = canonical.clone();

        let event_times = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));

        let mut watcher = notify::recommended_watcher({
            let event_times = event_times.clone();
            move |res| {
                match res {
                    Ok(event) => {
                        let kind = describe_event_kind(&event.kind);
                        for changed_path in event.paths.iter() {
                            if let Ok(stripped) = changed_path.strip_prefix(&base_path) {
                                // keep absolute path for consumers
                                let absolute = base_path.join(stripped);
                                if let Some(path_str) = absolute.to_str() {
                                    let path_string = path_str.to_string();
                                    if should_emit(&event_times, &path_string) {
                                        let payload = FsChangeEvent {
                                            kind: kind.clone(),
                                            path: path_string,
                                        };
                                        let _ = app_handle.emit("fs:changed", payload);
                                    }
                                }
                            } else if let Some(path_str) = changed_path.to_str() {
                                let path_string = path_str.to_string();
                                if should_emit(&event_times, &path_string) {
                                    let payload = FsChangeEvent {
                                        kind: kind.clone(),
                                        path: path_string,
                                    };
                                    let _ = app_handle.emit("fs:changed", payload);
                                }
                            }
                        }
                    }
                    Err(error) => {
                        let payload = FsChangeEvent {
                            kind: "error".into(),
                            path: format!("{error}"),
                        };
                        let _ = app_handle.emit("fs:changed", payload);
                    }
                }
            }
        })
        .map_err(|err| err.to_string())?;

        watcher
            .configure(Config::default().with_poll_interval(Duration::from_millis(200)))
            .map_err(|err| err.to_string())?;
        watcher
            .watch(&canonical, RecursiveMode::Recursive)
            .map_err(|err| err.to_string())?;

        watchers.insert(key, watcher);

        Ok(())
    }

    pub fn unwatch(&self, app: &AppHandle, path: String) -> Result<(), String> {
        let canonical = Self::canonical_path(&path)?;
        let key = canonical.to_string_lossy().to_string();

        app.fs_scope()
            .deny_directory(&canonical)
            .map_err(|err| err.to_string())?;

        let mut watchers = self
            .watchers
            .lock()
            .map_err(|_| "failed to lock fs watcher map".to_string())?;

        if let Some(mut watcher) = watchers.remove(&key) {
            watcher.unwatch(&canonical).map_err(|err| err.to_string())?;
        }

        Ok(())
    }

    fn canonical_path(path: &str) -> Result<PathBuf, String> {
        let path_buf = PathBuf::from(path);
        std::fs::canonicalize(&path_buf).map_err(|err| format!("failed to access {path}: {err}"))
    }
}

fn should_emit(times: &Arc<Mutex<HashMap<String, Instant>>>, path: &str) -> bool {
    let now = Instant::now();
    if let Ok(mut guard) = times.lock() {
        if let Some(previous) = guard.get(path) {
            if now.duration_since(*previous) < Duration::from_millis(200) {
                return false;
            }
        }
        guard.insert(path.to_string(), now);
        true
    } else {
        true
    }
}

fn describe_event_kind(kind: &EventKind) -> String {
    match kind {
        EventKind::Create(_) => "create".into(),
        EventKind::Modify(_) => "modify".into(),
        EventKind::Remove(_) => "remove".into(),
        EventKind::Access(_) => "access".into(),
        EventKind::Any => "any".into(),
        _ => "other".into(),
    }
}

#[tauri::command]
pub async fn register_project_root(
    app: AppHandle,
    manager: State<'_, FsWatcherManager>,
    path: String,
) -> Result<(), String> {
    manager.watch(app, path)
}

#[tauri::command]
pub async fn unregister_project_root(
    app: AppHandle,
    manager: State<'_, FsWatcherManager>,
    path: String,
) -> Result<(), String> {
    manager.unwatch(&app, path)
}
