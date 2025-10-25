use crate::git::models::{GitCommandConfig, GitError, GitExecutable, GitPathInfo};
use std::{path::PathBuf, sync::RwLock};

#[derive(Debug, Default)]
struct GitState {
    detected: Option<PathBuf>,
    configured: Option<PathBuf>,
}

pub struct GitService {
    state: RwLock<GitState>,
}

impl GitService {
    pub fn new() -> Self {
        let detected = Self::detect_system_git();
        Self {
            state: RwLock::new(GitState {
                detected,
                configured: None,
            }),
        }
    }

    pub fn refresh_detection(&self) {
        if let Ok(mut state) = self.state.write() {
            state.detected = Self::detect_system_git();
        }
    }

    pub fn set_override(&self, path: Option<String>) -> Result<GitPathInfo, GitError> {
        let mut state = self.state.write().unwrap();
        state.configured = match path {
            Some(value) => {
                if value.trim().is_empty() {
                    return Err(GitError::InvalidPath(
                        "git executable path cannot be empty".into(),
                    ));
                }
                let candidate = PathBuf::from(value);
                if !candidate.exists() {
                    return Err(GitError::InvalidPath(
                        "configured git executable path does not exist".into(),
                    ));
                }
                Some(candidate)
            }
            None => None,
        };
        Ok(self.info_inner(&state))
    }

    pub fn info(&self) -> GitPathInfo {
        let state = self.state.read().unwrap();
        self.info_inner(&state)
    }

    pub fn prepare(&self, repository_path: Option<&str>) -> Result<GitCommandConfig, GitError> {
        let executable = self.current_executable()?;
        let working_dir = match repository_path {
            Some(path) => crate::git::util::canonicalize_path(path)?,
            None => std::env::current_dir().map_err(|e| GitError::InvalidPath(e.to_string()))?,
        };
        Ok(GitCommandConfig {
            executable,
            working_dir,
        })
    }

    fn info_inner(&self, state: &GitState) -> GitPathInfo {
        let (effective, uses_wrapper) = self
            .resolve_executable(state)
            .map(|exec| (Some(exec.program_display()), exec.prefix_args.is_empty()))
            .unwrap_or((None, false));

        GitPathInfo {
            detected_path: state
                .detected
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            configured_path: state
                .configured
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            effective_path: effective,
            uses_wrapper: !uses_wrapper,
        }
    }

    fn current_executable(&self) -> Result<GitExecutable, GitError> {
        let state = self.state.read().unwrap();
        self.resolve_executable(&state)
    }

    fn resolve_executable(&self, state: &GitState) -> Result<GitExecutable, GitError> {
        if let Some(configured) = &state.configured {
            return Ok(GitExecutable {
                program: configured.to_string_lossy().to_string(),
                prefix_args: vec![],
                description: "user override".into(),
            });
        }

        if let Some(detected) = &state.detected {
            return Ok(GitExecutable {
                program: detected.to_string_lossy().to_string(),
                prefix_args: vec![],
                description: "detected git".into(),
            });
        }

        #[cfg(target_os = "windows")]
        {
            Ok(GitExecutable {
                program: "powershell.exe".into(),
                prefix_args: vec!["-NoProfile".into(), "-Command".into(), "git".into()],
                description: "powershell wrapper".into(),
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(GitExecutable {
                program: "git".into(),
                prefix_args: vec![],
                description: "git on PATH".into(),
            })
        }
    }

    fn detect_system_git() -> Option<PathBuf> {
        which::which("git").ok()
    }
}
