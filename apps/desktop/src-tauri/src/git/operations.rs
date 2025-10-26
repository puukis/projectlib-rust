use crate::git::{
    auth::{collect_cleanup, merge_auth_env, GitAuth},
    models::{
        GitBranchRequest, GitBranchesResponse, GitCheckoutRequest, GitCommandOutcome,
        GitCommitDetails, GitCommitDetailsRequest, GitCommitRequest, GitDeleteBranchRequest,
        GitError, GitErrorResponse, GitGraphResponse, GitLogResponse, GitPathInfo, GitRemoteList,
        GitRepositoryInfo, GitRepositoryRequest, GitStageRequest, GitStashApplyRequest,
        GitStashList, GitStashPushRequest, GitStatusResponse, GitStreamRequest, GitSwitchResponse,
    },
    service::GitService,
    streaming::run_streaming_command,
    util,
};
use log::{error, info};
use tauri::{AppHandle, State};
use tauri_plugin_shell::ShellExt;

async fn run_git_capture(
    app: &AppHandle,
    service: &GitService,
    repository_path: &str,
    mut args: Vec<String>,
    auth: Option<GitAuth>,
) -> Result<GitCommandOutcome, GitError> {
    let config = service.prepare(Some(repository_path))?;
    let display_args = args.clone();
    let mut command = app
        .shell()
        .command(&config.executable.program)
        .args(config.executable.prefix_args.clone())
        .args(args.drain(..))
        .current_dir(config.working_dir.clone());

    let mut cleanup = Vec::new();
    let mut env = std::collections::HashMap::new();
    if let Some(auth) = auth {
        let prepared = auth.prepare()?;
        env = merge_auth_env(env, &prepared.env);
        cleanup = collect_cleanup(cleanup, prepared.cleanup);
    }

    for (key, value) in env.iter() {
        command = command.env(key, value);
    }

    let output = command
        .output()
        .await
        .map_err(|e| GitError::Spawn(e.to_string()))?;
    drop(cleanup);

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let command_display = display_args.join(" ");
    if output.status.success() {
        info!("git:ok {}", command_display);
    } else {
        let code = output.status.code().unwrap_or(-1);
        error!("git:exit code={} {}", code, command_display);
    }

    Ok(GitCommandOutcome {
        exit_code: output.status.code(),
        success: output.status.success(),
        stdout,
        stderr,
    })
}

#[tauri::command]
pub fn git_path_info(service: State<'_, GitService>) -> GitPathInfo {
    service.info()
}

#[tauri::command]
pub fn git_set_path(
    service: State<'_, GitService>,
    path: Option<String>,
) -> Result<GitPathInfo, GitErrorResponse> {
    service.set_override(path).map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_detect_repository(
    _app: AppHandle,
    request: GitRepositoryRequest,
) -> Result<GitRepositoryInfo, GitErrorResponse> {
    let path = request.repository_path.ok_or_else(|| GitErrorResponse {
        message: "repository_path is required".into(),
    })?;
    let canonical = util::canonicalize_path(&path).map_err(GitErrorResponse::from)?;
    Ok(util::detect_repository(&canonical))
}

#[tauri::command]
pub async fn git_status(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitStatusResponse, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec![
            "status".into(),
            "--branch".into(),
            "--porcelain=v1".into(),
            "-z".into(),
        ],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_status(&outcome.stdout))
}

#[tauri::command]
pub async fn git_stage(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStageRequest,
) -> Result<GitCommandOutcome, GitErrorResponse> {
    if request.paths.is_empty() {
        return Err(GitErrorResponse {
            message: "no paths provided".into(),
        });
    }

    let mut args = vec!["add".into(), "--".into()];
    for path in request.paths.iter() {
        args.push(util::sanitize_arg(path, "path").map_err(GitErrorResponse::from)?);
    }

    run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_unstage(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStageRequest,
) -> Result<GitCommandOutcome, GitErrorResponse> {
    if request.paths.is_empty() {
        return Err(GitErrorResponse {
            message: "no paths provided".into(),
        });
    }

    let mut args = vec!["restore".into(), "--staged".into(), "--".into()];
    for path in request.paths.iter() {
        args.push(util::sanitize_arg(path, "path").map_err(GitErrorResponse::from)?);
    }

    run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_commit(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitCommitRequest,
) -> Result<GitCommandOutcome, GitErrorResponse> {
    if request.message.trim().is_empty() {
        return Err(GitErrorResponse {
            message: "commit message cannot be empty".into(),
        });
    }

    let mut args = vec!["commit".into(), "-m".into()];
    args.push(util::sanitize_arg(&request.message, "message").map_err(GitErrorResponse::from)?);

    run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_graph(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitGraphResponse, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec![
            "log".into(),
            "--date=iso-strict".into(),
            "--pretty=format:%H|%P|%an|%ad|%s".into(),
            "-n".into(),
            "200".into(),
        ],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_graph(&outcome.stdout))
}

#[tauri::command]
pub async fn git_commit_details(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitCommitDetailsRequest,
) -> Result<GitCommitDetails, GitErrorResponse> {
    let args = vec![
        "show".into(),
        "--name-status".into(),
        "--date=iso-strict".into(),
        "--pretty=format:%H%n%an%n%ad%n%B".into(),
        "--no-color".into(),
        util::sanitize_arg(&request.commit, "commit").map_err(GitErrorResponse::from)?,
    ];

    let outcome = run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)?;

    Ok(util::parse_commit_details(&outcome.stdout))
}

#[tauri::command]
pub async fn git_log(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitLogResponse, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec![
            "log".into(),
            "--oneline".into(),
            "--decorate".into(),
            "-n".into(),
            "100".into(),
            "--no-color".into(),
        ],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_log(&outcome.stdout))
}

#[tauri::command]
pub async fn git_branches(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitBranchesResponse, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec!["branch".into(), "-a".into(), "--no-color".into()],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_branches(&outcome.stdout))
}

#[tauri::command]
pub async fn git_switch_branch(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitBranchRequest,
) -> Result<GitSwitchResponse, GitErrorResponse> {
    let mut args = vec!["switch".into()];
    if request.create.unwrap_or(false) {
        args.push("-c".into());
    }
    if request.track.unwrap_or(false) {
        args.push("--track".into());
    }
    let branch = util::sanitize_arg(&request.branch, "branch").map_err(GitErrorResponse::from)?;
    args.push(branch.clone());

    let outcome = run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)?;

    if !outcome.success {
        return Err(GitErrorResponse {
            message: if outcome.stderr.is_empty() {
                "failed to switch branch".into()
            } else {
                outcome.stderr
            },
        });
    }

    Ok(GitSwitchResponse { branch })
}

#[tauri::command]
pub async fn git_delete_branch(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitDeleteBranchRequest,
) -> Result<GitSwitchResponse, GitErrorResponse> {
    let mut args = vec!["branch".into()];
    if request.force.unwrap_or(false) {
        args.push("-D".into());
    } else {
        args.push("-d".into());
    }
    let branch = util::sanitize_arg(&request.branch, "branch").map_err(GitErrorResponse::from)?;
    args.push(branch.clone());

    let outcome = run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)?;

    if !outcome.success {
        return Err(GitErrorResponse {
            message: if outcome.stderr.is_empty() {
                "failed to delete branch".into()
            } else {
                outcome.stderr
            },
        });
    }

    Ok(GitSwitchResponse { branch })
}

#[tauri::command]
pub async fn git_checkout(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitCheckoutRequest,
) -> Result<GitSwitchResponse, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &request.repository_path,
        vec![
            "checkout".into(),
            util::sanitize_arg(&request.target, "target").map_err(GitErrorResponse::from)?,
        ],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    if !outcome.success {
        return Err(GitErrorResponse {
            message: if outcome.stderr.is_empty() {
                "failed to checkout target".into()
            } else {
                outcome.stderr
            },
        });
    }

    Ok(GitSwitchResponse {
        branch: request.target,
    })
}

#[tauri::command]
pub async fn git_stash_list(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitStashList, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec![
            "stash".into(),
            "list".into(),
            "--pretty=format:%H%x01%gd%x01%cr%x01%s".into(),
        ],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_stash_list(&outcome.stdout))
}

#[tauri::command]
pub async fn git_stash_push(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStashPushRequest,
) -> Result<GitCommandOutcome, GitErrorResponse> {
    let mut args = vec!["stash".into(), "push".into()];
    if request.include_untracked.unwrap_or(false) {
        args.push("-u".into());
    }
    if let Some(message) = request.message.as_ref() {
        args.push("-m".into());
        args.push(message.clone());
    }

    run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_stash_apply(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStashApplyRequest,
) -> Result<GitCommandOutcome, GitErrorResponse> {
    let mut args = vec!["stash".into()];
    if request.drop.unwrap_or(false) {
        args.push("pop".into());
    } else {
        args.push("apply".into());
    }
    if let Some(name) = request.name.as_ref() {
        args.push(name.clone());
    }

    run_git_capture(&app, &service, &request.repository_path, args, None)
        .await
        .map_err(GitErrorResponse::from)
}

#[tauri::command]
pub async fn git_remote_list(
    app: AppHandle,
    service: State<'_, GitService>,
    repository_path: String,
) -> Result<GitRemoteList, GitErrorResponse> {
    let outcome = run_git_capture(
        &app,
        &service,
        &repository_path,
        vec!["remote".into(), "-v".into()],
        None,
    )
    .await
    .map_err(GitErrorResponse::from)?;

    Ok(util::parse_remotes(&outcome.stdout))
}

#[tauri::command]
pub async fn git_fetch_all(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStreamRequest,
) -> Result<crate::git::models::GitCommandHandle, GitErrorResponse> {
    run_streaming_command(app, service, request, vec!["fetch".into(), "--all".into()]).await
}

#[tauri::command]
pub async fn git_pull(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStreamRequest,
) -> Result<crate::git::models::GitCommandHandle, GitErrorResponse> {
    run_streaming_command(app, service, request, vec!["pull".into()]).await
}

#[tauri::command]
pub async fn git_push(
    app: AppHandle,
    service: State<'_, GitService>,
    request: GitStreamRequest,
) -> Result<crate::git::models::GitCommandHandle, GitErrorResponse> {
    run_streaming_command(app, service, request, vec!["push".into()]).await
}

#[tauri::command]
pub async fn git_run(
    app: AppHandle,
    service: State<'_, GitService>,
    cwd: String,
    args: Vec<String>,
) -> Result<(i32, String, String), String> {
    let joined = args.join(" ");
    let config = service.prepare(Some(&cwd)).map_err(|err| {
        let message = format!("git:err run {} @ {} -> {}", joined, cwd, err);
        error!("{message}");
        message
    })?;

    let mut command = app
        .shell()
        .command(&config.executable.program)
        .args(config.executable.prefix_args.clone())
        .args(args.clone())
        .current_dir(config.working_dir.clone());

    let output = command.output().await.map_err(|err| {
        let message = format!("git:err run {} @ {} -> {}", joined, cwd, err);
        error!("{message}");
        message
    })?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if code == 0 {
        info!("git:ok {}", joined);
    } else {
        error!("git:exit code={} {}", code, joined);
    }

    Ok((code, stdout, stderr))
}
