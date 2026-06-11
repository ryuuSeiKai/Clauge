use std::sync::Mutex;
use tokio::process::Command;

/// Synapse brand values injected into VS Code server's product.json.
const BRAND_SHORT: &str = "Synapse";
const BRAND_LONG: &str = "Synapse Editor";
const BRAND_URL_PROTOCOL: &str = "synapse-editor";
const BRAND_DATA_FOLDER: &str = ".synapse-editor";
const BRAND_SERVER_NAME: &str = "synapse-editor-server";
const BRAND_SERVER_DATA: &str = ".synapse-editor-server";

pub struct VscodeServer {
    process: Mutex<Option<tokio::process::Child>>,
    port: Mutex<u16>,
}

impl VscodeServer {
    pub fn new() -> Self {
        Self { process: Mutex::new(None), port: Mutex::new(8420) }
    }

    pub async fn start(&self, project_path: &str) -> Result<u16, String> {
        let mut port = 8420u16;
        let mut child = None;

        // Set up a server data directory that symlinks to the user's
        // desktop VS Code config so settings, keybindings, snippets,
        // themes, and extensions are shared automatically.
        let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
        let server_data = home.join(".synapse/vscode-server-data");
        std::fs::create_dir_all(&server_data)
            .map_err(|e| format!("Cannot create vscode-server-data dir: {}", e))?;
        setup_vscode_symlinks(&home, &server_data);
        write_transparent_theme(&server_data);
        let server_data_dir = server_data.to_string_lossy().to_string();

        // Patch VS Code server assets with Synapse branding (one-time).
        if let Some(assets) = find_server_assets(&home) {
            patch_product_json(&assets);
            patch_workbench_html(&assets);
        }

        // Resolve `code` binary — try common locations + PATH
        let code_bin = resolve_code_binary();
        if code_bin.is_none() {
            return Err("VS Code not found. Install VS Code and add `code` to PATH.".to_string());
        }
        let code_bin = code_bin.unwrap();

        for attempt in 0..10 {
            let test_port = port + attempt;
            let mut cmd = Command::new(&code_bin)
                .args([
                    "serve-web",
                    "--port", &test_port.to_string(),
                    "--without-connection-token",
                    "--accept-server-license-terms",
                    "--server-data-dir", &server_data_dir,
                    "--default-folder", project_path,
                ])
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| format!("Failed to start VS Code server: {}", e))?;

            // Brief wait to see if the process dies immediately (port conflict).
            // A port-conflict child exits almost instantly.
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            match cmd.try_wait() {
                Ok(None) => {
                    // Still alive — port was free.
                    child = Some(cmd);
                    port = test_port;
                    break;
                }
                _ => {
                    // Exited immediately — port likely in use, try next.
                    continue;
                }
            }
        }
        let child = child.ok_or_else(|| "All ports 8420-8429 are in use".to_string())?;
        *self.process.lock().unwrap() = Some(child);
        *self.port.lock().unwrap() = port;
        Ok(port)
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.start_kill();
        }
    }

    pub fn port(&self) -> u16 {
        *self.port.lock().unwrap()
    }
}

impl Drop for VscodeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Inject `<style>html,body{background:transparent!important}</style>` into
/// the workbench.html so the VS Code background layer is fully transparent.
/// Without this, VS Code's HTML body has a solid theme background that CSS
/// custom properties alone can't override.
fn patch_workbench_html(server_root: &std::path::Path) {
    let html_path = server_root
        .join("out/vs/code/browser/workbench/workbench.html");
    let bak_path = server_root
        .join("out/vs/code/browser/workbench/workbench.html.bak");

    if bak_path.exists() { return; }
    if !html_path.exists() { return; }

    let Ok(content) = std::fs::read_to_string(&html_path) else { return };

    // Back up original
    let _ = std::fs::write(&bak_path, &content);

    // Inject transparent CSS before </head>
    let patched = content.replace(
        "</head>",
        "<style>html,body{background:transparent!important}</style></head>"
    );
    let _ = std::fs::write(&html_path, patched);
    log::info!("Patched workbench.html for transparent background");
}

/// Patch the VS Code server's product.json with Synapse branding so the
/// embedded editor shows our name, uses our data folder, etc.
/// Only patches once — checks for existing backup first.
fn patch_product_json(server_root: &std::path::Path) {
    let product_path = server_root.join("product.json");
    let backup_path = server_root.join("product.json.bak");

    if backup_path.exists() {
        // Already patched
        return;
    }
    if !product_path.exists() {
        return;
    }

    let Ok(content) = std::fs::read_to_string(&product_path) else { return };
    let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) else { return };

    // Backup original
    let _ = std::fs::write(&backup_path, &content);

    // Apply Synapse branding
    if let Some(obj) = json.as_object_mut() {
        obj.insert("nameShort".into(), serde_json::Value::String(BRAND_SHORT.into()));
        obj.insert("nameLong".into(), serde_json::Value::String(BRAND_LONG.into()));
        obj.insert("applicationName".into(), serde_json::Value::String(BRAND_DATA_FOLDER.into()));
        obj.insert("dataFolderName".into(), serde_json::Value::String(BRAND_DATA_FOLDER.into()));
        obj.insert("urlProtocol".into(), serde_json::Value::String(BRAND_URL_PROTOCOL.into()));
        obj.insert("serverApplicationName".into(), serde_json::Value::String(BRAND_SERVER_NAME.into()));
        obj.insert("serverDataFolderName".into(), serde_json::Value::String(BRAND_SERVER_DATA.into()));
        // Use Synapse icon colours for the PWA manifest + favicon
        obj.insert("serverGreeting".into(), serde_json::Value::String("Synapse Editor".into()));

        let Ok(out) = serde_json::to_string_pretty(&obj) else { return };
        let _ = std::fs::write(&product_path, out);
        log::info!("Patched product.json for Synapse branding");
    }
}

/// Find the VS Code server web assets directory downloaded by the CLI.
fn find_server_assets(home: &std::path::Path) -> Option<std::path::PathBuf> {
    let serve_web = home.join(".vscode/cli/serve-web");
    if !serve_web.exists() { return None; }

    // Pick the first directory that has product.json — the server assets.
    let Ok(entries) = std::fs::read_dir(&serve_web) else { return None };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && p.join("product.json").exists() {
            return Some(p);
        }
    }
    None
}

/// Resolve the `code` CLI binary: check saved setting first, then common paths.
fn resolve_code_binary() -> Option<String> {
    // 1. Try saved setting from settings table (set via frontend)
    let mtx = CODE_BINARY_PATH.get_or_init(|| Mutex::new(None));
    let saved = mtx.lock().unwrap();
    if let Some(path) = saved.as_ref() {
        if std::path::Path::new(path).exists() || path == "code" {
            return Some(path.clone());
        }
    }
    drop(saved);

    // 2. Try common locations
    let candidates = vec![
        "code".to_string(),
        "/usr/local/bin/code".to_string(),
        "/opt/homebrew/bin/code".to_string(),
        "/usr/bin/code".to_string(),
    ];
    for bin in &candidates {
        if std::process::Command::new("sh")
            .args(["-c", &format!("command -v {} 2>/dev/null", bin)])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(bin.clone());
        }
    }
    // 3. Try VS Code .app bundle directly (macOS)
    let app_bundle = "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code";
    if std::path::Path::new(app_bundle).exists() {
        return Some(app_bundle.to_string());
    }
    None
}

/// Symlink the desktop VS Code `User/` and `extensions/` directories into
/// the server data dir so `serve-web` picks up the user's settings,
/// keybindings, snippets, themes, and (web-compatible) extensions.
fn setup_vscode_symlinks(home: &std::path::Path, server_data: &std::path::Path) {
    let extensions_dir = home.join(".vscode/extensions");

    // Symlink extensions — these are read-only and safe to share.
    symlink_or_skip(&extensions_dir, &server_data.join("extensions"), true);

    // Copy desktop settings.json + keybindings.json into server data/User/
    // on first run. Don't symlink the whole User/ dir — workspaceStorage
    // and globalStorage lock files would conflict with desktop VS Code.
    copy_desktop_settings(home, server_data);
}

/// Copy settings.json and keybindings.json from the desktop VS Code User
/// directory into the server's data/User/.  Does NOT overwrite — once the
/// embedded editor has its own copy, changes made there are preserved.
fn copy_desktop_settings(home: &std::path::Path, server_data: &std::path::Path) {
    let code_user = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Code/User")
    } else if cfg!(target_os = "linux") {
        home.join(".config/Code/User")
    } else {
        home.join("AppData/Roaming/Code/User")
    };

    let server_user = server_data.join("data").join("User");
    let _ = std::fs::create_dir_all(&server_user);

    for filename in &["settings.json", "keybindings.json"] {
        let src = code_user.join(filename);
        let dst = server_user.join(filename);
        if src.exists() && !dst.exists() {
            let _ = std::fs::copy(&src, &dst);
            log::info!("Copied {} to server data", filename);
        }
    }
}

/// Write a minimal Machine settings.json that makes VS Code backgrounds
/// transparent so the Synapse theme shows through the WebView.
fn write_transparent_theme(server_data: &std::path::Path) {
    let machine_dir = server_data.join("data").join("Machine");
    let _ = std::fs::create_dir_all(&machine_dir);
    let settings_path = machine_dir.join("settings.json");

    let accent = read_synapse_accent(server_data);
    let fg = accent.get("foreground").and_then(|v| v.as_str()).unwrap_or("#cccccc");
    let border = accent.get("border").and_then(|v| v.as_str()).unwrap_or("#ffffff20");
    let sel = accent.get("selection").and_then(|v| v.as_str()).unwrap_or("#ffffff15");
    let acc = accent.get("accent").and_then(|v| v.as_str()).unwrap_or("#6b8cff");

    // Only the SHELL chrome goes transparent — editor area, sidebar,
    // tabs, titlebar, statusbar.  Popups, menus, widgets, notifications
    // and inputs keep their solid backgrounds so they remain readable.
    let mut colors = serde_json::Map::new();
    let pairs: &[(&str, &str)] = &[
        // --- transparent shell ---
        ("editor.background", "#00000000"),
        ("sideBar.background", "#00000000"),
        ("activityBar.background", "#00000000"),
        ("editorGroupHeader.tabsBackground", "#00000000"),
        ("tab.inactiveBackground", "#00000000"),
        ("statusBar.background", "#00000000"),
        ("panel.background", "#00000000"),
        ("terminal.background", "#00000000"),
        ("titleBar.activeBackground", "#00000000"),
        ("titleBar.inactiveBackground", "#00000000"),
        ("breadcrumb.background", "#00000000"),
        ("editorGutter.background", "#00000000"),
        ("minimap.background", "#00000000"),
        ("editorStickyScroll.background", "#00000000"),
        ("editorOverviewRuler.background", "#00000000"),
        // --- subtle borders for definition ---
        ("sideBar.border", border),
        ("activityBar.border", border),
        ("editorGroupHeader.tabsBorder", border),
        ("tab.border", border),
        ("statusBar.border", border),
        ("panel.border", border),
        ("titleBar.border", border),
        ("editorWidget.border", border),
        ("input.border", border),
        ("tab.activeBorder", acc),
        // --- accent touches ---
        ("foreground", fg),
        ("focusBorder", acc),
        ("button.background", acc),
        ("badge.background", acc),
        ("progressBar.background", acc),
        ("tab.activeBorderTop", acc),
        // --- list / selection ---
        ("list.activeSelectionBackground", sel),
        ("list.hoverBackground", "#ffffff10"),
        // --- scrollbar ---
        ("scrollbarSlider.background", "#ffffff20"),
        ("scrollbarSlider.hoverBackground", "#ffffff30"),
        // --- line numbers ---
        ("editorLineNumber.foreground", "#ffffff35"),
        ("editorLineNumber.activeForeground", "#ffffff60"),
        // --- POPUPS / WIDGETS / NOTIFICATIONS — keep solid ---
        // Their backgrounds are intentionally NOT set here so VS Code
        // uses the theme defaults.
    ];
    for (k, v) in pairs {
        colors.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }

    let mut themed = serde_json::Map::new();
    themed.insert("[Default Dark Modern]".to_string(), serde_json::Value::Object(colors));
    let mut customizations = serde_json::Map::new();
    customizations.insert("workbench.colorCustomizations".to_string(), serde_json::Value::Object(themed));

    let Ok(json_str) = serde_json::to_string_pretty(&serde_json::Value::Object(customizations)) else { return };
    let _ = std::fs::write(&settings_path, json_str);
    log::info!("Wrote transparent Machine settings to {:?}", settings_path);
}

/// Read `synapse_theme.json` from the server data root.  The frontend
/// writes this file on editor init with accent/foreground/border/selection
/// colours extracted from the active Synapse CSS custom properties.
fn read_synapse_accent(server_data: &std::path::Path) -> serde_json::Value {
    let path = server_data.join("synapse_theme.json");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
            return v;
        }
    }
    serde_json::json!({})
}

/// Ensure `dst` is a symlink to `src`. If `dst` already exists as a regular
/// directory it is removed first when `force_replace` is true.
fn symlink_or_skip(src: &std::path::Path, dst: &std::path::Path, force_replace: bool) {
    if !src.exists() { return; }

    if dst.exists() {
        if dst.is_symlink() { return; } // already done
        if force_replace && dst.is_dir() {
            let _ = std::fs::remove_dir_all(dst);
        } else {
            return;
        }
    }

    #[cfg(unix)]
    { let _ = std::os::unix::fs::symlink(src, dst); }
    #[cfg(windows)]
    { let _ = std::os::windows::fs::symlink_dir(src, dst); }
    log::info!("Symlinked {:?} -> {:?}", dst, src);
}

static CODE_BINARY_PATH: std::sync::OnceLock<Mutex<Option<String>>> = std::sync::OnceLock::new();

/// Set a custom VS Code binary path from the frontend settings.
/// Pass empty string to reset to auto-detection.
pub fn set_code_binary_path(path: &str) {
    let mtx = CODE_BINARY_PATH.get_or_init(|| Mutex::new(None));
    let mut saved = mtx.lock().unwrap();
    if path.is_empty() {
        *saved = None;
    } else {
        *saved = Some(path.to_string());
    }
}

#[tauri::command]
pub fn editor_set_binary_path(path: String) -> Result<(), String> {
    set_code_binary_path(&path);
    Ok(())
}

#[tauri::command]
pub fn editor_get_port(state: tauri::State<'_, VscodeServer>) -> Result<u16, String> {
    Ok(state.port())
}

#[tauri::command]
pub fn editor_sync_theme(theme_json: String) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let server_data = home.join(".synapse/vscode-server-data");
    let _ = std::fs::create_dir_all(&server_data);
    let path = server_data.join("synapse_theme.json");
    std::fs::write(&path, &theme_json).map_err(|e| e.to_string())?;
    log::info!("Synced Synapse theme to {:?}", path);
    Ok(())
}

#[tauri::command]
pub async fn editor_open_project(
    state: tauri::State<'_, VscodeServer>,
    project_path: String,
) -> Result<u16, String> {
    state.stop();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let path = if project_path.trim().is_empty() {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        project_path
    };
    state.start(&path).await
}
