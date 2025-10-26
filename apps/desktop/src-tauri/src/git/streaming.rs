use crate::git::{
    auth::{collect_cleanup, merge_auth_env},
    models::{
        GitCommandHandle, GitError, GitErrorResponse, GitStreamEvent, GitStreamEventKind,
        GitStreamRequest,
    },
    service::GitService,
    util,
};
use std::collections::HashMap;
use tauri::{AppHandle, State};
use tauri::Emitter;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use uuid::Uuid;

pub const STREAM_EVENT: &str = "git://stream";

pub async fn run_streaming_command(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStreamRequest,
    mut args: Vec<String>,
) -> Result<GitCommandHandle, GitErrorResponse> {
    if let Some(remote) = request.remote.as_ref() {
        args.push(util::sanitize_arg(remote, "remote").map_err(GitErrorResponse::from)?);
    }
    if let Some(branch) = request.branch.as_ref() {
        args.push(util::sanitize_arg(branch, "branch").map_err(GitErrorResponse::from)?);
    }

    let config = service
        .prepare(Some(&request.repository_path))
        .map_err(GitErrorResponse::from)?;

    let mut command = app
        .shell()
        .command(&config.executable.program)
        .args(config.executable.prefix_args.clone())
        .args(args)
        .current_dir(config.working_dir.clone());

    let mut cleanup = Vec::new();
    let mut env: HashMap<String, String> = HashMap::new();

    if let Some(auth) = request.auth {
        let prepared = auth.prepare().map_err(GitErrorResponse::from)?;
        env = merge_auth_env(env, &prepared.env);
        cleanup = collect_cleanup(cleanup, prepared.cleanup);
    }

    for (key, value) in env.iter() {
        command = command.env(key, value);
    }

    let (mut rx, _child) = command
        .spawn()
        .map_err(|e| GitErrorResponse::from(GitError::Spawn(e.to_string())))?;

    let command_id = request
        .command_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let event_name = STREAM_EVENT.to_string();
    let app_handle = app.clone();
    let stream_command_id = command_id.clone();

    tauri::async_runtime::spawn(async move {
        let _cleanup_guard = cleanup;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let payload = GitStreamEvent {
                        command_id: stream_command_id.clone(),
                        kind: GitStreamEventKind::Stdout,
                        data: Some(String::from_utf8_lossy(&line).to_string()),
                        exit_code: None,
                        success: None,
                    };
                    let _ = app_handle.emit(&event_name, payload);
                }
                CommandEvent::Stderr(line) => {
                    let payload = GitStreamEvent {
                        command_id: stream_command_id.clone(),
                        kind: GitStreamEventKind::Stderr,
                        data: Some(String::from_utf8_lossy(&line).to_string()),
                        exit_code: None,
                        success: None,
                    };
                    let _ = app_handle.emit(&event_name, payload);
                }
                CommandEvent::Terminated(payload) => {
                    let success = payload.code.unwrap_or(-1) == 0;
                    let payload = GitStreamEvent {
                        command_id: stream_command_id.clone(),
                        kind: GitStreamEventKind::Completed,
                        data: None,
                        exit_code: payload.code,
                        success: Some(success),
                    };
                    let _ = app_handle.emit(&event_name, payload);
                }
                CommandEvent::Error(message) => {
                    let payload = GitStreamEvent {
                        command_id: stream_command_id.clone(),
                        kind: GitStreamEventKind::Error,
                        data: Some(message),
                        exit_code: None,
                        success: None,
                    };
                    let _ = app_handle.emit(&event_name, payload);
                }
                _ => {}
            }
        }
    });

    Ok(GitCommandHandle { command_id })
}
