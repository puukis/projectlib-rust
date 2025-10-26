use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ShellInfo {
    pub program: String,
    pub args: Vec<String>,
}

#[tauri::command]
pub fn terminal_default_shell() -> Result<ShellInfo, String> {
    default_shell().map_err(|err| err.to_string())
}

#[cfg(target_os = "macos")]
fn default_shell() -> Result<ShellInfo, std::io::Error> {
    use std::env;

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    Ok(ShellInfo {
        program: "/usr/bin/env".to_string(),
        args: vec![shell],
    })
}

#[cfg(target_os = "windows")]
fn default_shell() -> Result<ShellInfo, std::io::Error> {
    use which::which;

    let pwsh = which("pwsh.exe").or_else(|_| which("pwsh"));
    if let Ok(path) = pwsh {
        return Ok(ShellInfo {
            program: path.to_string_lossy().into_owned(),
            args: vec!["-NoLogo".to_string()],
        });
    }

    let cmd_path = which("cmd.exe").unwrap_or_else(|_| PathBuf::from("cmd.exe"));
    Ok(ShellInfo {
        program: cmd_path.to_string_lossy().into_owned(),
        args: vec!["/K".to_string()],
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn default_shell() -> Result<ShellInfo, std::io::Error> {
    use std::env;

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    Ok(ShellInfo {
        program: shell,
        args: Vec::new(),
    })
}
