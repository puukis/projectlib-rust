use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunSuggestion {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<String>,
    pub reason: String,
}

#[tauri::command]
pub fn detect_project_runs(path: String) -> Result<Vec<RunSuggestion>, String> {
    let root = PathBuf::from(path);

    if !root.exists() {
        return Err("Path does not exist".into());
    }

    if !root.is_dir() {
        return Err("Path must point to a directory".into());
    }

    let mut suggestions: Vec<RunSuggestion> = Vec::new();

    detect_node(&root, &mut suggestions);
    detect_python(&root, &mut suggestions);
    detect_gradle(&root, &mut suggestions);
    detect_rust(&root, &mut suggestions);
    detect_go(&root, &mut suggestions);
    detect_dotnet(&root, &mut suggestions);
    detect_c_like(&root, &mut suggestions);

    Ok(suggestions)
}

fn detect_node(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return;
    }

    let Ok(contents) = fs::read_to_string(&package_json) else {
        return;
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return;
    };

    let Some(scripts) = value.get("scripts").and_then(|s| s.as_object()) else {
        return;
    };

    for candidate in ["dev", "start"] {
        if scripts.contains_key(candidate) {
            let suggestion = RunSuggestion {
                language: "node".into(),
                command: "pnpm".into(),
                args: vec!["run".into(), candidate.to_string()],
                env: HashMap::new(),
                cwd: Some(path_to_string(root)),
                reason: format!(
                    "Detected package.json with `{}` script",
                    candidate
                ),
            };
            push_unique(suggestions, suggestion);
            break;
        }
    }
}

fn detect_python(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    let has_pyproject = root.join("pyproject.toml").exists();
    let has_requirements = root.join("requirements.txt").exists();

    if !has_pyproject && !has_requirements {
        return;
    }

    let module_name = if has_pyproject {
        parse_python_module(root).or_else(|| default_module_name(root))
    } else {
        default_module_name(root)
    };

    let Some(module) = module_name else {
        return;
    };

    let uses_uv = root.join("uv.lock").exists() || root.join("uv.toml").exists();
    let command;
    let mut args = Vec::new();
    let reason;

    if uses_uv {
        command = "uv".to_string();
        args.push("run".into());
        args.push("python".into());
        args.push("-m".into());
        reason = "Detected pyproject with uv configuration".to_string();
    } else {
        command = "python".to_string();
        args.push("-m".into());
        reason = if has_pyproject {
            "Detected pyproject.toml".to_string()
        } else {
            "Detected requirements.txt".to_string()
        };
    }

    args.push(module);

    let suggestion = RunSuggestion {
        language: "python".into(),
        command,
        args,
        env: HashMap::new(),
        cwd: Some(path_to_string(root)),
        reason,
    };

    push_unique(suggestions, suggestion);
}

fn parse_python_module(root: &Path) -> Option<String> {
    let pyproject_path = root.join("pyproject.toml");
    let contents = fs::read_to_string(pyproject_path).ok()?;
    let value: toml::Value = toml::from_str(&contents).ok()?;

    if let Some(name) = value
        .get("project")
        .and_then(|project| project.get("name"))
        .and_then(|name| name.as_str())
    {
        return Some(name.replace('-', "_"));
    }

    value
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("name"))
        .and_then(|name| name.as_str())
        .map(|name| name.replace('-', "_"))
}

fn default_module_name(root: &Path) -> Option<String> {
    root.file_name()
        .and_then(|os| os.to_str())
        .map(|name| name.replace('-', "_"))
}

fn detect_gradle(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    if !(root.join("gradlew").exists() || root.join("gradlew.bat").exists()) {
        return;
    }

    let suggestion = RunSuggestion {
        language: "java".into(),
        command: "./gradlew".into(),
        args: vec!["run".into()],
        env: HashMap::new(),
        cwd: Some(path_to_string(root)),
        reason: "Detected Gradle wrapper".into(),
    };

    push_unique(suggestions, suggestion);
}

fn detect_rust(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    if !root.join("Cargo.toml").exists() {
        return;
    }

    let suggestion = RunSuggestion {
        language: "rust".into(),
        command: "cargo".into(),
        args: vec!["run".into()],
        env: HashMap::new(),
        cwd: Some(path_to_string(root)),
        reason: "Detected Cargo.toml".into(),
    };

    push_unique(suggestions, suggestion);
}

fn detect_go(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    if !root.join("go.mod").exists() {
        return;
    }

    let suggestion = RunSuggestion {
        language: "go".into(),
        command: "go".into(),
        args: vec!["run".into(), ".".into()],
        env: HashMap::new(),
        cwd: Some(path_to_string(root)),
        reason: "Detected go.mod".into(),
    };

    push_unique(suggestions, suggestion);
}

fn detect_dotnet(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    let mut csproj_dir: Option<PathBuf> = None;

    for entry in WalkDir::new(root).max_depth(3).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("csproj"))
                .unwrap_or(false)
        {
            csproj_dir = entry.path().parent().map(Path::to_path_buf);
            break;
        }
    }

    let Some(dir) = csproj_dir else {
        return;
    };

    let suggestion = RunSuggestion {
        language: ".net".into(),
        command: "dotnet".into(),
        args: vec!["run".into()],
        env: HashMap::new(),
        cwd: Some(path_to_string(&dir)),
        reason: "Detected .csproj file".into(),
    };

    push_unique(suggestions, suggestion);
}

fn detect_c_like(root: &Path, suggestions: &mut Vec<RunSuggestion>) {
    if root.join("Makefile").exists() {
        let suggestion = RunSuggestion {
            language: "c".into(),
            command: "make".into(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: Some(path_to_string(root)),
            reason: "Detected Makefile".into(),
        };
        push_unique(suggestions, suggestion);
    }

    if root.join("CMakeLists.txt").exists() {
        let suggestion = RunSuggestion {
            language: "c".into(),
            command: "cmake".into(),
            args: vec!["--build".into(), "build".into()],
            env: HashMap::new(),
            cwd: Some(path_to_string(root)),
            reason: "Detected CMakeLists.txt".into(),
        };
        push_unique(suggestions, suggestion);
    }
}

fn push_unique(suggestions: &mut Vec<RunSuggestion>, suggestion: RunSuggestion) {
    if suggestions
        .iter()
        .any(|existing| existing.command == suggestion.command && existing.args == suggestion.args)
    {
        return;
    }

    suggestions.push(suggestion);
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
