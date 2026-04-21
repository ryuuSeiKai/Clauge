pub mod models;
pub use models::*;

pub mod storage;
use storage::*;

pub mod profiles;
pub mod git;
pub mod worktree;
pub mod terminal;
pub mod plugins;
pub mod usage;

use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{TrayIconBuilder, TrayIconId};
use tauri::Manager;

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Get all profiles — no auto-discovery of session IDs
/// Session IDs are only set when explicitly captured after a session starts
#[tauri::command]
fn refresh_session_ids() -> Result<Vec<SessionProfile>, String> {
    Ok(load_profiles())
}

/// Update the claude session ID for a specific profile
#[tauri::command]
fn update_session_id(id: String, claude_session_id: String) -> Result<(), String> {
    let mut profiles = load_profiles();
    if let Some(profile) = profiles.iter_mut().find(|p| p.id == id) {
        profile.claude_session_id = Some(claude_session_id);
    } else {
        return Err("Profile not found".to_string());
    }
    save_profiles(&profiles)?;
    Ok(())
}

/// Count active sessions for a project path (profiles that have been used)
#[tauri::command]
fn count_project_sessions(project_path: String) -> Result<u32, String> {
    let profiles = load_profiles();
    let count = profiles.iter()
        .filter(|p| p.project_path == project_path)
        .count() as u32;
    Ok(count)
}

#[tauri::command]
fn discover_sessions(project_path: String) -> Result<Vec<DiscoveredSession>, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(&project_path);
    let projects_dir = home.join(".claude").join("projects").join(&encoded);

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    let entries = std::fs::read_dir(&projects_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let modified_at = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            // Extract first user message as preview
            let preview = std::fs::read_to_string(&path).ok().and_then(|content| {
                for line in content.lines().take(20) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        if val.get("type").and_then(|t| t.as_str()) == Some("human") {
                            if let Some(msg) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                let trimmed = msg.chars().take(80).collect::<String>();
                                return Some(trimmed);
                            }
                        }
                    }
                }
                None
            });

            sessions.push(DiscoveredSession {
                session_id,
                modified_at,
                preview,
            });
        }
    }

    sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(sessions)
}

#[tauri::command]
fn get_session_tokens(
    project_path: String,
    session_id: Option<String>,
) -> Result<TokenUsage, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(&project_path);
    let projects_dir = home.join(".claude").join("projects").join(&encoded);

    if !projects_dir.exists() {
        return Err("Project directory not found".to_string());
    }

    let file_path = if let Some(sid) = session_id {
        projects_dir.join(format!("{}.jsonl", sid))
    } else {
        // Find most recent .jsonl file
        let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if best.as_ref().map_or(true, |(_, t)| modified > *t) {
                                best = Some((path, modified));
                            }
                        }
                    }
                }
            }
        }
        best.map(|(p, _)| p)
            .ok_or("No session files found")?
    };

    if !file_path.exists() {
        return Err("Session file not found".to_string());
    }

    let contents = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut cache_read_tokens: u64 = 0;
    let mut cache_creation_tokens: u64 = 0;

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            // Check both direct usage and message.usage patterns
            let usage = val.get("usage").or_else(|| {
                val.get("message").and_then(|m| m.get("usage"))
            });
            if let Some(u) = usage {
                input_tokens += u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                output_tokens += u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                cache_read_tokens += u.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                cache_creation_tokens += u.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }

    let total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens;

    Ok(TokenUsage {
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_creation_tokens,
        total_tokens,
    })
}


/// Save session key to local storage
#[tauri::command]
fn save_session_key(key: String) -> Result<(), String> {
    let path = get_storage_dir().join("session_key");
    std::fs::write(&path, &key).map_err(|e| e.to_string())
}

/// Load session key from local storage
#[tauri::command]
fn load_session_key() -> Result<Option<String>, String> {
    let path = get_storage_dir().join("session_key");
    if path.exists() {
        let key = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let key = key.trim().to_string();
        if key.is_empty() { Ok(None) } else { Ok(Some(key)) }
    } else {
        Ok(None)
    }
}

/// Get app version from Cargo.toml
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get Claude subscription plan from keychain
#[tauri::command]
fn get_claude_plan() -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .map_err(|e| format!("Keychain error: {}", e))?;
    if !output.status.success() { return Ok(String::new()); }
    let json_str = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(json_str.trim()).map_err(|e| e.to_string())?;
    Ok(parsed.get("claudeAiOauth")
        .and_then(|o| o.get("subscriptionType").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string())
}

/// Update the tray title text (shown in menu bar)
#[tauri::command]
fn update_tray_title(app_handle: tauri::AppHandle, title: String) -> Result<(), String> {
    if let Some(tray) = app_handle.tray_by_id(&TrayIconId::new("main-tray")) {
        tray.set_title(Some(&title)).map_err(|e| format!("Tray title error: {}", e))?;
    }
    Ok(())
}


/// Get storage dir for context snippets
fn get_contexts_dir() -> PathBuf {
    let dir = get_storage_dir().join("contexts");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// List all saved context snippets
#[tauri::command]
fn get_context_snippets() -> Result<Vec<serde_json::Value>, String> {
    let dir = get_contexts_dir();
    let mut snippets = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let preview = content.lines().next().unwrap_or("").chars().take(80).collect::<String>();
                snippets.push(serde_json::json!({
                    "name": name,
                    "content": content,
                    "preview": preview,
                }));
            }
        }
    }
    snippets.sort_by(|a, b| a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or("")));
    Ok(snippets)
}

/// Save a context snippet
#[tauri::command]
fn save_context_snippet(name: String, content: String) -> Result<(), String> {
    let path = get_contexts_dir().join(format!("{}.md", name));
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}

/// Delete a context snippet
#[tauri::command]
fn delete_context_snippet(name: String) -> Result<(), String> {
    let path = get_contexts_dir().join(format!("{}.md", name));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Inject context snippets into CLAUDE.md for a session
#[tauri::command]
fn inject_session_context(project_path: String, context_names: Vec<String>) -> Result<(), String> {
    let contexts_dir = get_contexts_dir();
    let mut combined = String::new();

    for name in &context_names {
        let path = contexts_dir.join(format!("{}.md", name));
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !combined.is_empty() { combined.push_str("\n\n---\n\n"); }
            combined.push_str(&format!("## {}\n\n{}", name, content));
        }
    }

    let claude_md_path = PathBuf::from(&project_path).join("CLAUDE.md");
    let marker_start = "<!-- CLAUGE-CONTEXT-START -->";
    let marker_end = "<!-- CLAUGE-CONTEXT-END -->";

    // Read existing content (without old markers) to check for duplicates
    let existing_content = if claude_md_path.exists() {
        let raw = std::fs::read_to_string(&claude_md_path).map_err(|e| e.to_string())?;
        if let (Some(start), Some(end)) = (raw.find(marker_start), raw.find(marker_end)) {
            raw[..start].trim_end().to_string()
        } else {
            raw
        }
    } else {
        String::new()
    };

    // Filter out snippets whose content already exists in the file
    let mut filtered = String::new();
    for name in &context_names {
        let path = contexts_dir.join(format!("{}.md", name));
        if let Ok(content) = std::fs::read_to_string(&path) {
            if !existing_content.contains(content.trim()) {
                if !filtered.is_empty() { filtered.push_str("\n\n---\n\n"); }
                filtered.push_str(&format!("## {}\n\n{}", name, content));
            }
        }
    }

    if filtered.is_empty() { return Ok(()); }

    let injected = format!("\n\n{}\n{}\n{}\n", marker_start, filtered, marker_end);

    if !existing_content.is_empty() {
        std::fs::write(&claude_md_path, format!("{}{}", existing_content.trim_end(), injected)).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(&claude_md_path, filtered).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Remove injected context from CLAUDE.md
#[tauri::command]
fn remove_injected_context(project_path: String) -> Result<(), String> {
    let claude_md_path = PathBuf::from(&project_path).join("CLAUDE.md");
    if !claude_md_path.exists() { return Ok(()); }

    let content = std::fs::read_to_string(&claude_md_path).map_err(|e| e.to_string())?;
    let marker_start = "<!-- CLAUGE-CONTEXT-START -->";
    let marker_end = "<!-- CLAUGE-CONTEXT-END -->";

    if let (Some(start), Some(end)) = (content.find(marker_start), content.find(marker_end)) {
        let cleaned = format!("{}{}", &content[..start].trim_end(), &content[end + marker_end.len()..]);
        if cleaned.trim().is_empty() {
            // We created this file — delete it
            let _ = std::fs::remove_file(&claude_md_path);
        } else {
            std::fs::write(&claude_md_path, cleaned.trim_end().to_string() + "\n").map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Update contexts attached to a session profile
#[tauri::command]
fn update_session_contexts(id: String, contexts: Vec<String>) -> Result<(), String> {
    let mut profiles = load_profiles();
    if let Some(profile) = profiles.iter_mut().find(|p| p.id == id) {
        profile.contexts = contexts;
    } else {
        return Err("Profile not found".to_string());
    }
    save_profiles(&profiles)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            profiles::get_profiles,
            profiles::create_profile,
            profiles::delete_profile,
            profiles::rename_profile,
            profiles::update_last_used,
            refresh_session_ids,
            update_session_id,
            worktree::is_git_repo,
            git::get_git_status,
            git::get_git_branch,
            git::get_git_ahead_behind,
            git::git_commit,
            git::git_push,
            git::git_pull,
            git::git_diff_file,
            git::git_stage_file,
            git::git_unstage_file,
            git::git_log,
            git::git_stash,
            git::git_stash_pop,
            git::git_list_branches,
            git::git_switch_branch,
            worktree::create_worktree,
            worktree::remove_worktree,
            worktree::update_profile_worktree,
            count_project_sessions,
            discover_sessions,
            get_session_tokens,
            usage::fetch_usage_limits,
            usage::get_usage_analytics,
            get_app_version,
            get_claude_plan,
            update_tray_title,
            save_session_key,
            load_session_key,
            terminal::spawn_terminal,
            terminal::spawn_shell,
            terminal::write_to_terminal,
            terminal::resize_terminal,
            terminal::kill_terminal,
            plugins::get_claude_plugins,
            plugins::toggle_claude_plugin,
            plugins::get_marketplace_plugins,
            plugins::install_plugin,
            plugins::uninstall_plugin,
            get_context_snippets,
            save_context_snippet,
            delete_context_snippet,
            inject_session_context,
            remove_injected_context,
            update_session_contexts
        ])
        .setup(|app| {
            let setup_start = std::time::Instant::now();
            eprintln!("[TIMING] setup start");

            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None)
                    .expect("Failed to apply vibrancy");
            }
            eprintln!("[TIMING] vibrancy applied: {:?}", setup_start.elapsed());

            // ---- App menu bar ----
            let app_menu = Submenu::with_items(app, "Clauge", true, &[
                &PredefinedMenuItem::about(app, Some("About Clauge"), None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::show_all(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::quit(app, None)?,
            ])?;
            let edit_menu = Submenu::with_items(app, "Edit", true, &[
                &PredefinedMenuItem::undo(app, None)?,
                &PredefinedMenuItem::redo(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::cut(app, None)?,
                &PredefinedMenuItem::copy(app, None)?,
                &PredefinedMenuItem::paste(app, None)?,
                &PredefinedMenuItem::select_all(app, None)?,
            ])?;
            let window_menu = Submenu::with_items(app, "Window", true, &[
                &PredefinedMenuItem::minimize(app, None)?,
                &PredefinedMenuItem::maximize(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::close_window(app, None)?,
            ])?;
            let menu_bar = Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])?;
            app.set_menu(menu_bar)?;

            // ---- System tray ----
            let show_item = MenuItem::with_id(app, "show", "Back to App", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show_item, &separator, &quit])?;

            // Use custom tray icon — template mode so macOS adapts to light/dark menu bar
            let icon_png = include_bytes!("../icons/tray-dark.png");
            let img = image::load_from_memory(icon_png).expect("Failed to load tray icon");
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            let tray_icon = tauri::image::Image::new_owned(rgba.into_raw(), w, h);
            TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .title("Clauge")
                .tooltip("Clauge — Claude Session Manager")
                .on_menu_event(move |app: &tauri::AppHandle, event: tauri::menu::MenuEvent| {
                    let id = event.id().as_ref();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "show" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            eprintln!("[TIMING] tray built: {:?}", setup_start.elapsed());

            // Enable autostart on first run
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();

            eprintln!("[TIMING] setup complete: {:?}", setup_start.elapsed());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of quit — user can quit from tray
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::Reopen { .. } => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                tauri::RunEvent::ExitRequested { .. } => {
                    if let Some(state) = app.try_state::<TerminalState>() {
                        let mut terminals = state.terminals.lock();
                        for (id, mut entry) in terminals.drain() {
                            let _ = entry.child.kill();
                            eprintln!("[Clauge] Cleaned up terminal {} on exit", id);
                        }
                    }
                }
                _ => {}
            }
        });
}
