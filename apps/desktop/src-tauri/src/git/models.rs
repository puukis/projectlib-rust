use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git executable not available")]
    MissingGit,
    #[error("invalid path provided: {0}")]
    InvalidPath(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to spawn git: {0}")]
    Spawn(String),
    #[error("failed to parse git output: {0}")]
    Parse(String),
    #[error("the repository path is required")]
    MissingRepository,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitErrorResponse {
    pub message: String,
}

impl From<GitError> for GitErrorResponse {
    fn from(value: GitError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitExecutable {
    pub program: String,
    pub prefix_args: Vec<String>,
    pub description: String,
}

impl GitExecutable {
    pub fn program_display(&self) -> String {
        if self.prefix_args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.prefix_args.join(" "))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitPathInfo {
    pub detected_path: Option<String>,
    pub configured_path: Option<String>,
    pub effective_path: Option<String>,
    pub uses_wrapper: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryInfo {
    pub is_repository: bool,
    pub worktree_root: Option<String>,
    pub git_dir: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusResponse {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    pub staged: Vec<GitFileChange>,
    pub unstaged: Vec<GitFileChange>,
    pub conflicts: Vec<GitFileChange>,
    pub untracked: Vec<String>,
    pub is_clean: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    pub path: String,
    pub original_path: Option<String>,
    pub index_status: Option<String>,
    pub worktree_status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitLogEntry {
    pub commit: String,
    pub refs: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitLogResponse {
    pub entries: Vec<GitLogEntry>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchesResponse {
    pub current: Option<String>,
    pub local: Vec<String>,
    pub remote: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitSwitchResponse {
    pub branch: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStashEntry {
    pub name: String,
    pub hash: String,
    pub relative_time: String,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStashList {
    pub entries: Vec<GitStashEntry>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub url: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteList {
    pub remotes: Vec<GitRemote>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandHandle {
    pub command_id: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandCompletion {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitCommandOutcome {
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitStreamEventKind {
    Stdout,
    Stderr,
    Completed,
    Error,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStreamEvent {
    pub command_id: String,
    pub kind: GitStreamEventKind,
    pub data: Option<String>,
    pub exit_code: Option<i32>,
    pub success: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRepositoryRequest {
    pub repository_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchRequest {
    pub repository_path: String,
    pub branch: String,
    pub create: Option<bool>,
    pub track: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCheckoutRequest {
    pub repository_path: String,
    pub target: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashPushRequest {
    pub repository_path: String,
    pub message: Option<String>,
    pub include_untracked: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStashApplyRequest {
    pub repository_path: String,
    pub name: Option<String>,
    pub drop: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRemoteRequest {
    pub repository_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStreamRequest {
    pub repository_path: String,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub auth: Option<crate::git::auth::GitAuth>,
    pub command_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitCommandConfig {
    pub executable: GitExecutable,
    pub working_dir: PathBuf,
}
