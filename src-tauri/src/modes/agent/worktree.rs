use std::path::PathBuf;

fn sanitize_branch_name(name: &str) -> String {
    let sanitized: String = name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '/' || *c == '-' || *c == '_' || *c == '.')
        .collect();
    let sanitized = sanitized.replace("..", ".").replace(".lock", "")
        .trim_matches(|c: char| c == '.' || c == '/' || c == '-').to_string();
    if sanitized.is_empty() { return "clauge/unnamed".to_string(); }
    sanitized.split('/').map(|seg| {
        if seg.starts_with('-') { format!("x{}", seg) } else { seg.to_string() }
    }).collect::<Vec<_>>().join("/")
}

#[tauri::command]
pub fn agent_is_git_repo(path: String) -> Result<bool, String> {
    let output = std::process::Command::new("git").args(["-C", &path, "rev-parse", "--is-inside-work-tree"]).output().map_err(|e| e.to_string())?;
    Ok(output.status.success())
}

#[tauri::command]
pub fn agent_create_worktree(project_path: String, branch_name: String) -> Result<String, String> {
    let branch_name = sanitize_branch_name(&branch_name);
    let worktree_dir = PathBuf::from(&project_path).join(".clauge-worktrees").join(&branch_name);
    let worktree_path = worktree_dir.to_string_lossy().to_string();
    if worktree_dir.exists() { return Ok(worktree_path); }
    let _ = std::fs::create_dir_all(worktree_dir.parent().unwrap_or(&worktree_dir));
    let _ = std::process::Command::new("git").args(["-C", &project_path, "worktree", "prune"]).output();
    let output = std::process::Command::new("git").args(["-C", &project_path, "worktree", "add", "-b", &branch_name, &worktree_path]).output().map_err(|e| format!("git worktree add failed: {}", e))?;
    if !output.status.success() {
        let output2 = std::process::Command::new("git").args(["-C", &project_path, "worktree", "add", &worktree_path, &branch_name]).output().map_err(|e| format!("git worktree add failed: {}", e))?;
        if !output2.status.success() { return Err(format!("git worktree add failed: {}", String::from_utf8_lossy(&output2.stderr))); }
    }
    let gitignore = PathBuf::from(&project_path).join(".gitignore");
    if let Ok(contents) = std::fs::read_to_string(&gitignore) {
        if !contents.contains(".clauge-worktrees") {
            let _ = std::fs::write(&gitignore, format!("{}\n.clauge-worktrees/\n", contents.trim_end()));
        }
    } else {
        let _ = std::fs::write(&gitignore, ".clauge-worktrees/\n");
    }
    Ok(worktree_path)
}

#[tauri::command]
pub fn agent_remove_worktree(project_path: String, worktree_path: String) -> Result<(), String> {
    use crate::shared::platform::path::{apply_user_path, find_binary};
    let git_bin = find_binary("git").ok_or_else(|| "git is not installed or not on PATH".to_string())?;

    let mut remove = std::process::Command::new(&git_bin);
    apply_user_path(&mut remove);
    let out = remove
        .args(["-C", &project_path, "worktree", "remove", "--force", &worktree_path])
        .output()
        .map_err(|e| format!("git worktree remove failed to spawn: {e}"))?;

    let mut prune = std::process::Command::new(&git_bin);
    apply_user_path(&mut prune);
    let _ = prune
        .args(["-C", &project_path, "worktree", "prune"])
        .output();

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // Treat "not a working tree" / "no such directory" as success — the
        // worktree is already gone (deleted outside Clauge); prune above
        // cleared the stale git metadata. Caller's intent is satisfied.
        let lower = stderr.to_lowercase();
        if lower.contains("is not a working tree")
            || lower.contains("no such file or directory")
            || lower.contains("not a valid working tree")
        {
            return Ok(());
        }
        return Err(if stderr.is_empty() {
            "git worktree remove failed with no output".to_string()
        } else {
            stderr
        });
    }
    Ok(())
}

/// True when the worktree at `worktree_path` has uncommitted changes
/// (modified, staged, or untracked). Used as a preflight before the
/// destructive `git worktree remove --force` in session-delete so we
/// can warn the user that committing-or-stashing now would save work
/// that's about to be discarded.
#[tauri::command]
pub fn agent_worktree_is_dirty(worktree_path: String) -> Result<bool, String> {
    use crate::shared::platform::path::{apply_user_path, find_binary};
    let git_bin = find_binary("git").ok_or_else(|| "git is not installed or not on PATH".to_string())?;
    let mut cmd = std::process::Command::new(&git_bin);
    apply_user_path(&mut cmd);
    let out = cmd
        .args(["-C", &worktree_path, "status", "--porcelain"])
        .output()
        .map_err(|e| format!("git status failed to spawn: {e}"))?;
    if !out.status.success() {
        // Worktree path doesn't exist / isn't a git checkout. Treat as
        // "not dirty" so the delete flow doesn't block on a missing
        // worktree — the user wants it gone either way.
        return Ok(false);
    }
    Ok(!String::from_utf8_lossy(&out.stdout).trim().is_empty())
}
