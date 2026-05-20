use crate::modes::agent::models::GitFileChange;

#[tauri::command]
pub fn agent_git_status(project_path: String) -> Result<Vec<GitFileChange>, String> {
    let output = std::process::Command::new("git")
        .args(["-C", &project_path, "status", "--porcelain", "-u"])
        .output().map_err(|e| format!("git status failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(|line| {
        let status = line[..2].trim().to_string();
        let path = line[3..].to_string();
        GitFileChange { path, status }
    }).collect())
}

#[tauri::command]
pub fn agent_git_branch(project_path: String) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["-C", &project_path, "rev-parse", "--abbrev-ref", "HEAD"])
        .output().map_err(|e| format!("git branch failed: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn agent_git_ahead_behind(project_path: String) -> Result<(u32, u32), String> {
    let output = std::process::Command::new("git")
        .args(["-C", &project_path, "rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output().map_err(|e| format!("git rev-list failed: {}", e))?;
    if !output.status.success() { return Ok((0, 0)); }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = stdout.split_whitespace().collect();
    let ahead = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let behind = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok((ahead, behind))
}

#[tauri::command]
pub fn agent_git_commit(project_path: String, message: String) -> Result<String, String> {
    let add = std::process::Command::new("git").args(["-C", &project_path, "add", "-A"]).output().map_err(|e| format!("git add failed: {}", e))?;
    if !add.status.success() { return Err(format!("git add failed: {}", String::from_utf8_lossy(&add.stderr))); }
    let commit = std::process::Command::new("git").args(["-C", &project_path, "commit", "-m", &message]).output().map_err(|e| format!("git commit failed: {}", e))?;
    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr).trim().to_string();
        if stderr.contains("nothing to commit") { return Ok("Nothing to commit".to_string()); }
        return Err(format!("git commit failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&commit.stdout).trim().to_string())
}

#[tauri::command]
pub fn agent_git_push(project_path: String) -> Result<String, String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "push"]).output().map_err(|e| format!("git push failed: {}", e))?;
    if !output.status.success() {
        let output2 = std::process::Command::new("git").args(["-C", &project_path, "push", "--set-upstream", "origin", "HEAD"]).output().map_err(|e| format!("git push failed: {}", e))?;
        if !output2.status.success() { return Err(format!("git push failed: {}", String::from_utf8_lossy(&output2.stderr))); }
        return Ok(String::from_utf8_lossy(&output2.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
}

#[tauri::command]
pub fn agent_git_pull(project_path: String) -> Result<String, String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "pull"]).output().map_err(|e| format!("git pull failed: {}", e))?;
    if !output.status.success() { return Err(format!("git pull failed: {}", String::from_utf8_lossy(&output.stderr))); }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn agent_git_diff_file(project_path: String, file_path: String) -> Result<String, String> {
    let staged = std::process::Command::new("git").args(["-C", &project_path, "diff", "--cached", "--", &file_path]).output().map_err(|e| format!("git diff failed: {}", e))?;
    let staged_out = String::from_utf8_lossy(&staged.stdout).to_string();
    let unstaged = std::process::Command::new("git").args(["-C", &project_path, "diff", "--", &file_path]).output().map_err(|e| format!("git diff failed: {}", e))?;
    let unstaged_out = String::from_utf8_lossy(&unstaged.stdout).to_string();
    if staged_out.is_empty() && unstaged_out.is_empty() {
        let full_path = std::path::PathBuf::from(&project_path).join(&file_path);
        if full_path.exists() {
            let content = std::fs::read_to_string(&full_path).unwrap_or_default();
            return Ok(format!("New file: {}\n\n{}", file_path, content));
        }
    }
    if !staged_out.is_empty() { Ok(staged_out) } else { Ok(unstaged_out) }
}

#[tauri::command]
pub fn agent_git_stage_file(project_path: String, file_path: String) -> Result<(), String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "add", "--", &file_path]).output().map_err(|e| format!("git add failed: {}", e))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    Ok(())
}

#[tauri::command]
pub fn agent_git_unstage_file(project_path: String, file_path: String) -> Result<(), String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "restore", "--staged", "--", &file_path]).output().map_err(|e| format!("git restore failed: {}", e))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    Ok(())
}

#[tauri::command]
pub fn agent_git_log(project_path: String, limit: Option<u32>) -> Result<Vec<serde_json::Value>, String> {
    let n = limit.unwrap_or(10).to_string();
    let output = std::process::Command::new("git").args(["-C", &project_path, "log", &format!("-{}", n), "--pretty=format:%H|||%h|||%s|||%an|||%ar"]).output().map_err(|e| format!("git log failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.splitn(5, "|||").collect();
        serde_json::json!({ "hash": parts.get(0).unwrap_or(&""), "short": parts.get(1).unwrap_or(&""), "message": parts.get(2).unwrap_or(&""), "author": parts.get(3).unwrap_or(&""), "date": parts.get(4).unwrap_or(&"") })
    }).collect())
}

#[tauri::command]
pub fn agent_git_stash(project_path: String) -> Result<String, String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "stash"]).output().map_err(|e| format!("git stash failed: {}", e))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn agent_git_stash_pop(project_path: String) -> Result<String, String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "stash", "pop"]).output().map_err(|e| format!("git stash pop failed: {}", e))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub fn agent_git_list_branches(project_path: String) -> Result<Vec<serde_json::Value>, String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "branch", "--format=%(refname:short)|||%(HEAD)"]).output().map_err(|e| format!("git branch failed: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().filter(|l| !l.is_empty()).map(|line| {
        let parts: Vec<&str> = line.splitn(2, "|||").collect();
        serde_json::json!({ "name": parts.get(0).unwrap_or(&"").trim(), "current": parts.get(1).unwrap_or(&"").trim() == "*" })
    }).collect())
}

#[tauri::command]
pub fn agent_git_switch_branch(project_path: String, branch_name: String) -> Result<(), String> {
    let output = std::process::Command::new("git").args(["-C", &project_path, "checkout", &branch_name]).output().map_err(|e| format!("git checkout failed: {}", e))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_string()); }
    Ok(())
}
