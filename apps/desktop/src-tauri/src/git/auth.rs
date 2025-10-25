use crate::git::models::GitError;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Write as _};
use tempfile::{Builder, TempPath};

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GitAuth {
    Token {
        token: String,
        username: Option<String>,
    },
    UserPassword {
        username: String,
        password: String,
    },
    SshCommand {
        command: String,
    },
}

#[derive(Debug, Default)]
pub struct PreparedAuth {
    pub env: HashMap<String, String>,
    pub cleanup: Vec<TempPath>,
}

impl PreparedAuth {
    pub fn empty() -> Self {
        Self {
            env: HashMap::new(),
            cleanup: Vec::new(),
        }
    }
}

impl GitAuth {
    pub fn prepare(&self) -> Result<PreparedAuth, GitError> {
        match self {
            GitAuth::Token { token, username } => {
                let username = username.clone().unwrap_or_else(|| "git".to_string());
                Self::prepare_askpass(&username, token)
            }
            GitAuth::UserPassword { username, password } => {
                Self::prepare_askpass(username, password)
            }
            GitAuth::SshCommand { command } => {
                if command.trim().is_empty() {
                    return Err(GitError::InvalidArgument(
                        "SSH command override must not be empty".into(),
                    ));
                }
                let mut env = HashMap::new();
                env.insert("GIT_SSH_COMMAND".into(), command.clone());
                Ok(PreparedAuth {
                    env,
                    cleanup: vec![],
                })
            }
        }
    }

    fn prepare_askpass(username: &str, secret: &str) -> Result<PreparedAuth, GitError> {
        if username.contains('\0') || secret.contains('\0') {
            return Err(GitError::InvalidArgument(
                "credential values may not contain null bytes".into(),
            ));
        }

        let mut env = HashMap::new();
        env.insert("TAURI_GIT_USERNAME".into(), username.to_string());
        env.insert("TAURI_GIT_PASSWORD".into(), secret.to_string());
        env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());

        #[cfg(target_os = "windows")]
        let (script, temp_path) = Self::windows_script()?;
        #[cfg(not(target_os = "windows"))]
        let (script, temp_path) = Self::unix_script()?;

        std::fs::write(&script, Self::askpass_contents())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script)?.permissions();
            perms.set_mode(0o700);
            std::fs::set_permissions(&script, perms)?;
        }

        env.insert("GIT_ASKPASS".into(), script.to_string_lossy().to_string());

        Ok(PreparedAuth {
            env,
            cleanup: vec![temp_path],
        })
    }

    #[cfg(not(target_os = "windows"))]
    fn unix_script() -> Result<(std::path::PathBuf, TempPath), GitError> {
        let file = Builder::new().prefix("projectlib-askpass-").tempfile()?;
        let path = file.path().to_path_buf();
        Ok((path, file.into_temp_path()))
    }

    #[cfg(target_os = "windows")]
    fn windows_script() -> Result<(std::path::PathBuf, TempPath), GitError> {
        let file = Builder::new()
            .prefix("projectlib-askpass-")
            .suffix(".cmd")
            .tempfile()?;
        let path = file.path().to_path_buf();
        Ok((path, file.into_temp_path()))
    }

    fn askpass_contents() -> String {
        #[cfg(target_os = "windows")]
        {
            let mut content = String::new();
            writeln!(&mut content, "@echo off").unwrap();
            writeln!(&mut content, "set prompt=%1").unwrap();
            writeln!(&mut content, "echo %prompt%| findstr /I \"Username\" >nul").unwrap();
            writeln!(&mut content, "if %errorlevel%==0 (").unwrap();
            writeln!(&mut content, "  echo %TAURI_GIT_USERNAME%").unwrap();
            writeln!(&mut content, ") else (").unwrap();
            writeln!(&mut content, "  echo %TAURI_GIT_PASSWORD%").unwrap();
            writeln!(&mut content, ")").unwrap();
            content
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut content = String::new();
            writeln!(&mut content, "#!/bin/sh").unwrap();
            writeln!(
                &mut content,
                "case \"$1\" in\n  *Username* ) printf '%s\\n' \"$TAURI_GIT_USERNAME\" ;;\n  *username* ) printf '%s\\n' \"$TAURI_GIT_USERNAME\" ;;\n  * ) printf '%s\\n' \"$TAURI_GIT_PASSWORD\" ;;\nesac"
            )
            .unwrap();
            content
        }
    }
}

pub fn merge_auth_env(
    mut base: HashMap<String, String>,
    extra: &HashMap<String, String>,
) -> HashMap<String, String> {
    for (key, value) in extra {
        base.insert(key.clone(), value.clone());
    }
    base
}

pub fn collect_cleanup(mut existing: Vec<TempPath>, mut extra: Vec<TempPath>) -> Vec<TempPath> {
    existing.append(&mut extra);
    existing
}
