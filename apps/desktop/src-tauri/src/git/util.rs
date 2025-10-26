use crate::git::models::{
    GitBranchesResponse, GitCommitDetails, GitCommitFileChange, GitError, GitFileChange,
    GitGraphEntry, GitGraphResponse, GitLogEntry, GitLogResponse, GitRemote, GitRemoteList,
    GitRepositoryInfo, GitStashEntry, GitStashList, GitStatusResponse,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub fn sanitize_arg(value: &str, field: &str) -> Result<String, GitError> {
    if value.is_empty() {
        return Err(GitError::InvalidArgument(format!(
            "{field} cannot be empty"
        )));
    }
    if value.contains('\0') {
        return Err(GitError::InvalidArgument(format!(
            "{field} may not contain null bytes"
        )));
    }
    Ok(value.to_string())
}

pub fn canonicalize_path(path: &str) -> Result<PathBuf, GitError> {
    if path.trim().is_empty() {
        return Err(GitError::InvalidPath("path cannot be empty".into()));
    }
    let path_buf = PathBuf::from(path);
    let dir = if path_buf.is_dir() {
        path_buf
    } else {
        path_buf
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| GitError::InvalidPath("path does not exist".into()))?
    };

    fs::canonicalize(dir.clone()).map_err(|_| GitError::InvalidPath("path does not exist".into()))
}

pub fn detect_repository(path: &Path) -> GitRepositoryInfo {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    };

    loop {
        let git_dir = current.join(".git");
        if git_dir.is_dir() {
            return GitRepositoryInfo {
                is_repository: true,
                worktree_root: Some(current.to_string_lossy().to_string()),
                git_dir: Some(git_dir.to_string_lossy().to_string()),
            };
        }
        if git_dir.is_file() {
            if let Ok(contents) = fs::read_to_string(&git_dir) {
                if let Some(dir_line) = contents.lines().find(|line| line.starts_with("gitdir:")) {
                    let path_part = dir_line.trim_start_matches("gitdir:").trim();
                    let git_path = PathBuf::from(path_part);
                    let abs_git = if git_path.is_absolute() {
                        git_path
                    } else {
                        current.join(git_path)
                    };
                    return GitRepositoryInfo {
                        is_repository: true,
                        worktree_root: Some(current.to_string_lossy().to_string()),
                        git_dir: Some(abs_git.to_string_lossy().to_string()),
                    };
                }
            }
        }

        if !current.pop() {
            break;
        }
    }

    GitRepositoryInfo {
        is_repository: false,
        worktree_root: None,
        git_dir: None,
    }
}

pub fn parse_status(output: &str) -> GitStatusResponse {
    let mut branch = None;
    let mut upstream = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut detached = false;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut conflicts = Vec::new();
    let mut untracked = Vec::new();

    let mut entries = output.split('\0').peekable();
    while let Some(entry) = entries.next() {
        if entry.is_empty() {
            continue;
        }

        if let Some(rest) = entry.strip_prefix("## ") {
            parse_branch_line(
                rest,
                &mut branch,
                &mut upstream,
                &mut ahead,
                &mut behind,
                &mut detached,
            );
            continue;
        }

        if entry.starts_with("?? ") {
            if let Some(path) = entry.get(3..) {
                if !path.is_empty() {
                    untracked.push(path.to_string());
                }
            }
            continue;
        }

        if entry.len() <= 3 {
            continue;
        }

        let status = &entry[0..2];
        let mut path_section = entry[3..].to_string();
        let mut original_path: Option<String> = None;

        if status.starts_with('R') || status.starts_with('C') {
            if let Some(next_path) = entries.next() {
                original_path = Some(path_section.clone());
                path_section = next_path.to_string();
            }
        }

        let (path, parsed_original) = parse_path(&path_section);
        if original_path.is_none() {
            original_path = parsed_original;
        }

        let index_status = match status.chars().next() {
            Some(' ') | Some('?') => None,
            Some(c) => Some(c.to_string()),
            None => None,
        };
        let worktree_status = match status.chars().nth(1) {
            Some(' ') | Some('?') => None,
            Some(c) => Some(c.to_string()),
            None => None,
        };

        let change = GitFileChange {
            path: path.clone(),
            original_path: original_path.clone(),
            index_status: index_status.clone(),
            worktree_status: worktree_status.clone(),
        };

        if status.contains('U') {
            conflicts.push(change);
        } else {
            if index_status.is_some() {
                staged.push(change.clone());
            }
            if worktree_status.is_some() {
                unstaged.push(change);
            }
        }
    }

    let is_clean =
        staged.is_empty() && unstaged.is_empty() && untracked.is_empty() && conflicts.is_empty();

    GitStatusResponse {
        branch,
        upstream,
        ahead,
        behind,
        detached,
        staged,
        unstaged,
        conflicts,
        untracked,
        is_clean,
    }
}

fn parse_branch_line(
    line: &str,
    branch: &mut Option<String>,
    upstream: &mut Option<String>,
    ahead: &mut u32,
    behind: &mut u32,
    detached: &mut bool,
) {
    let mut names_part = line;
    let mut summary_part = "";
    if let Some(idx) = line.find('[') {
        names_part = &line[..idx];
        summary_part = &line[idx..];
    }

    if let Some((head, rest)) = names_part.split_once("...") {
        if head.contains("(no branch)") || head.contains("HEAD") {
            *detached = true;
        } else {
            *branch = Some(head.trim().to_string());
        }
        if !rest.trim().is_empty() {
            *upstream = Some(rest.trim().to_string());
        }
    } else {
        let trimmed = names_part.trim();
        if trimmed.starts_with("HEAD") {
            *detached = true;
        } else if !trimmed.is_empty() {
            *branch = Some(trimmed.to_string());
        }
    }

    if summary_part.starts_with('[') {
        let details = summary_part.trim_matches(|c| c == '[' || c == ']');
        for part in details.split(',') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("ahead ") {
                if let Ok(num) = value.parse::<u32>() {
                    *ahead = num;
                }
            } else if let Some(value) = part.strip_prefix("behind ") {
                if let Ok(num) = value.parse::<u32>() {
                    *behind = num;
                }
            }
        }
    }
}

fn parse_path(path_section: &str) -> (String, Option<String>) {
    if let Some(idx) = path_section.find(" -> ") {
        let (from, to) = path_section.split_at(idx);
        let to = to.trim_start_matches(" -> ");
        (to.to_string(), Some(from.to_string()))
    } else {
        (path_section.to_string(), None)
    }
}

pub fn parse_log(output: &str) -> GitLogResponse {
    let entries = output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let commit = parts.next()?.to_string();
            let rest = parts.next().unwrap_or("").trim();
            let (refs, summary) = if rest.starts_with('(') {
                if let Some(end_idx) = rest.find(')') {
                    let refs_part = &rest[1..end_idx];
                    let refs = refs_part
                        .split(',')
                        .map(|r| r.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let summary = rest[end_idx + 1..].trim().to_string();
                    (refs, summary)
                } else {
                    (Vec::new(), rest.to_string())
                }
            } else {
                (Vec::new(), rest.to_string())
            };
            Some(GitLogEntry {
                commit,
                refs,
                summary,
            })
        })
        .collect();

    GitLogResponse { entries }
}

pub fn parse_branches(output: &str) -> GitBranchesResponse {
    let mut current = None;
    let mut local = Vec::new();
    let mut remote = Vec::new();
    let mut seen = HashSet::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_current = trimmed.starts_with('*');
        let name = trimmed.trim_start_matches('*').trim();
        if name.starts_with("remotes/") {
            if seen.insert(name.to_string()) {
                remote.push(name.to_string());
            }
        } else if seen.insert(name.to_string()) {
            local.push(name.to_string());
        }
        if is_current {
            current = Some(name.to_string());
        }
    }

    GitBranchesResponse {
        current,
        local,
        remote,
    }
}

pub fn parse_remotes(output: &str) -> GitRemoteList {
    let mut remotes = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            remotes.push(GitRemote {
                name: parts[0].to_string(),
                url: parts[1].to_string(),
                kind: parts[2].trim_matches(|c| c == '(' || c == ')').to_string(),
            });
        }
    }
    GitRemoteList { remotes }
}

pub fn parse_stash_list(output: &str) -> GitStashList {
    let mut entries = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split('\u{0001}').collect();
        if parts.len() >= 4 {
            entries.push(GitStashEntry {
                hash: parts[0].to_string(),
                name: parts[1].to_string(),
                relative_time: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }
    GitStashList { entries }
}

pub fn parse_graph(output: &str) -> GitGraphResponse {
    let entries = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut parts = trimmed.splitn(5, '|');
            let commit = parts.next()?.to_string();
            let parents_part = parts.next().unwrap_or("");
            let author = parts.next().unwrap_or("").to_string();
            let date = parts.next().unwrap_or("").to_string();
            let subject = parts.next().unwrap_or("").to_string();
            let parents = if parents_part.trim().is_empty() {
                Vec::new()
            } else {
                parents_part
                    .split_whitespace()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
            };

            Some(GitGraphEntry {
                commit,
                parents,
                author,
                date,
                subject,
            })
        })
        .collect();

    GitGraphResponse { entries }
}

pub fn parse_commit_details(output: &str) -> GitCommitDetails {
    let mut lines = output.lines();
    let commit = lines.next().unwrap_or("").to_string();
    let author = lines.next().unwrap_or("").to_string();
    let date = lines.next().unwrap_or("").to_string();

    let mut message_lines = Vec::new();
    for line in lines.by_ref() {
        if line.trim().is_empty() {
            break;
        }
        message_lines.push(line.to_string());
    }

    let message = message_lines.join("\n").trim().to_string();
    let mut files = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let status = parts.next().unwrap_or("").to_string();
        let path = parts.collect::<Vec<&str>>().join(" ");
        if status.is_empty() || path.is_empty() {
            continue;
        }
        files.push(GitCommitFileChange { status, path });
    }

    GitCommitDetails {
        commit,
        author,
        date,
        message,
        files,
    }
}
