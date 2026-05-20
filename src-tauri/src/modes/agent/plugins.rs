use crate::modes::agent::models::{ClaudePlugin, MarketplacePlugin};
use crate::shared::cli::{registry::runner_for, runner::CliRunner};
use std::fs;

// Each CLI exposes plugins differently. Claude is marketplace-directory
// driven (~/.claude/plugins/marketplaces/) and uses `claude plugins
// <args>` for install/uninstall. Codex's plugin lifecycle lives inside
// its TUI (`/plugins` slash command); the only on-disk record is
// `[plugins."<name>@<marketplace>"] enabled = ...` blocks in
// ~/.codex/config.toml. OpenCode uses npm packages — no marketplace
// concept — and is excluded from the manager entirely.
//
// Top-level commands dispatch off the `provider` arg so the right
// model runs for each CLI.

fn resolved(provider: Option<&str>) -> &'static dyn CliRunner {
    runner_for(provider.unwrap_or("claude"))
}

fn provider_id(p: Option<&str>) -> &str {
    p.unwrap_or("claude")
}

#[tauri::command]
pub fn agent_get_plugins(provider: Option<String>) -> Result<Vec<ClaudePlugin>, String> {
    match provider_id(provider.as_deref()) {
        "codex" => codex::list_installed(),
        _ => claude_style::list_installed(resolved(provider.as_deref())),
    }
}

#[tauri::command]
pub fn agent_toggle_plugin(
    provider: Option<String>,
    plugin_key: String,
    enabled: bool,
) -> Result<(), String> {
    match provider_id(provider.as_deref()) {
        "codex" => codex::set_enabled(&plugin_key, enabled),
        _ => claude_style::set_enabled(resolved(provider.as_deref()), &plugin_key, enabled),
    }
}

#[tauri::command]
pub fn agent_get_marketplace_plugins(
    provider: Option<String>,
) -> Result<Vec<MarketplacePlugin>, String> {
    match provider_id(provider.as_deref()) {
        // Codex marketplace browsing isn't wired yet — plugins ship as
        // git-repo marketplaces with TOML manifests, deferred.
        "codex" => Ok(Vec::new()),
        _ => claude_style::list_marketplace(resolved(provider.as_deref())),
    }
}

#[tauri::command]
pub fn agent_install_plugin(
    provider: Option<String>,
    name: String,
    marketplace: String,
) -> Result<(), String> {
    match provider_id(provider.as_deref()) {
        // Codex doesn't expose plugin install via shell CLI — the
        // `/plugins` slash command inside an interactive codex session
        // is the supported install path. Surface a clean error instead
        // of corrupting config.toml.
        "codex" => Err(
            "Codex plugins install inside the Codex TUI. Open `codex` and run `/plugins`.".into(),
        ),
        _ => claude_style::install(resolved(provider.as_deref()), &name, &marketplace),
    }
}

#[tauri::command]
pub fn agent_uninstall_plugin(
    provider: Option<String>,
    name: String,
    marketplace: String,
) -> Result<(), String> {
    match provider_id(provider.as_deref()) {
        "codex" => codex::uninstall(&name, &marketplace),
        _ => claude_style::uninstall(resolved(provider.as_deref()), &name, &marketplace),
    }
}

// ─── Claude-style (directory + CLI subcommand) ────────────────────────
//
// Used by any runner that exposes `installed_plugins.json` +
// `marketplaces/` + a `<cli> plugins <args>` interface. OpenCode opts
// out by returning `None` from the runner's `installed_plugins_file`
// hook — list_installed returns an empty Vec rather than erroring, so
// the frontend can simply hide an empty list.

mod claude_style {
    use super::*;

    pub fn list_installed(cli: &'static dyn CliRunner) -> Result<Vec<ClaudePlugin>, String> {
        let settings_path = match cli.settings_file() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let installed_path = match cli.installed_plugins_file() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let mut enabled_map: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
            let settings: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            if let Some(plugins) = settings.get("enabledPlugins").and_then(|v| v.as_object()) {
                for (key, val) in plugins { enabled_map.insert(key.clone(), val.as_bool().unwrap_or(false)); }
            }
        }
        let mut plugins = Vec::new();
        if installed_path.exists() {
            let content = fs::read_to_string(&installed_path).map_err(|e| e.to_string())?;
            let installed: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
            if let Some(plugin_map) = installed.get("plugins").and_then(|v| v.as_object()) {
                for (key, entries) in plugin_map {
                    let parts: Vec<&str> = key.splitn(2, '@').collect();
                    let name = parts.first().unwrap_or(&"").to_string();
                    let marketplace = parts.get(1).unwrap_or(&"").to_string();
                    let (version, install_path) = entries.as_array().and_then(|arr| arr.first()).map(|entry| {
                        (entry.get("version").and_then(|v| v.as_str()).map(String::from), entry.get("installPath").and_then(|v| v.as_str()).map(String::from))
                    }).unwrap_or((None, None));
                    let enabled = enabled_map.get(key).copied().unwrap_or(false);
                    plugins.push(ClaudePlugin { name, marketplace, enabled, version, install_path });
                }
            }
        }
        plugins.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(plugins)
    }

    pub fn set_enabled(
        cli: &'static dyn CliRunner,
        plugin_key: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let settings_path = cli
            .settings_file()
            .ok_or("This CLI does not expose a plugin settings file")?;
        let mut settings: serde_json::Value = if settings_path.exists() {
            serde_json::from_str(&fs::read_to_string(&settings_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        } else {
            serde_json::json!({})
        };
        if settings.get("enabledPlugins").is_none() {
            settings["enabledPlugins"] = serde_json::json!({});
        }
        settings["enabledPlugins"][plugin_key] = serde_json::Value::Bool(enabled);
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_marketplace(cli: &'static dyn CliRunner) -> Result<Vec<MarketplacePlugin>, String> {
        let marketplaces_dir = match cli.plugin_marketplaces_dir() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let installed_path = cli.installed_plugins_file().ok_or("Cannot determine home directory")?;
        let mut installed_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
        if installed_path.exists() {
            if let Ok(content) = fs::read_to_string(&installed_path) {
                if let Ok(installed) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(plugins) = installed.get("plugins").and_then(|v| v.as_object()) {
                        for key in plugins.keys() { installed_keys.insert(key.clone()); }
                    }
                }
            }
        }
        let mut install_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let counts_path = cli.plugin_install_counts_file().ok_or("Cannot determine home directory")?;
        if counts_path.exists() {
            if let Ok(content) = fs::read_to_string(&counts_path) {
                if let Ok(cache) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(counts) = cache.get("counts").and_then(|v| v.as_array()) {
                        for entry in counts {
                            if let (Some(plugin), Some(count)) = (entry.get("plugin").and_then(|v| v.as_str()), entry.get("unique_installs").and_then(|v| v.as_u64())) {
                                install_counts.insert(plugin.to_string(), count);
                            }
                        }
                    }
                }
            }
        }
        let mut results = Vec::new();
        if !marketplaces_dir.exists() { return Ok(results); }
        for entry in fs::read_dir(&marketplaces_dir).map_err(|e| e.to_string())?.flatten() {
            let marketplace_name = entry.file_name().to_string_lossy().to_string();
            let registry_path = entry.path().join(".claude-plugin").join("marketplace.json");
            if !registry_path.exists() { continue; }
            let content = match fs::read_to_string(&registry_path) { Ok(c) => c, Err(_) => continue };
            let registry: serde_json::Value = match serde_json::from_str(&content) { Ok(v) => v, Err(_) => continue };
            if let Some(plugins) = registry.get("plugins").and_then(|v| v.as_array()) {
                for plugin in plugins {
                    let name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if name.is_empty() { continue; }
                    let description = plugin.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let category = plugin.get("category").and_then(|v| v.as_str()).map(String::from);
                    let key = format!("{}@{}", name, marketplace_name);
                    results.push(MarketplacePlugin { name, description, marketplace: marketplace_name.clone(), category, installed: installed_keys.contains(&key), installs: install_counts.get(&key).copied() });
                }
            }
        }
        results.sort_by(|a, b| b.installs.unwrap_or(0).cmp(&a.installs.unwrap_or(0)));
        Ok(results)
    }

    pub fn install(
        cli: &'static dyn CliRunner,
        name: &str,
        marketplace: &str,
    ) -> Result<(), String> {
        let plugin_id = format!("{}@{}", name, marketplace);
        let (ok, stderr) = cli.run_plugin_subcommand(&["install", &plugin_id])?;
        if !ok {
            return Err(format!("Install failed: {}", stderr));
        }
        Ok(())
    }

    pub fn uninstall(
        cli: &'static dyn CliRunner,
        name: &str,
        marketplace: &str,
    ) -> Result<(), String> {
        let plugin_id = format!("{}@{}", name, marketplace);
        let (ok, stderr) = cli.run_plugin_subcommand(&["uninstall", &plugin_id])?;
        if !ok {
            return Err(format!("Uninstall failed: {}", stderr));
        }
        Ok(())
    }
}

// ─── Codex (TOML config blocks under ~/.codex/config.toml) ─────────────
//
// Codex enumerates installed plugins as TOML tables keyed by
// `<name>@<marketplace>`:
//
//     [plugins."gmail@openai-curated"]
//     enabled = true
//     # ... other metadata codex stores per plugin
//
// We read those table headers to list, and edit the `enabled` field
// in place to toggle. We use `toml_edit` (format-preserving) so the
// user's other config (project trust levels, tui settings, etc.) is
// untouched. Install/uninstall happen inside the codex TUI's
// `/plugins` browser — surfaced as an instructional message by the
// dispatcher above.

mod codex {
    use super::*;
    use toml_edit::{DocumentMut, Item, Value};

    fn config_path() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|h| h.join(".codex").join("config.toml"))
    }

    fn read_doc() -> Result<DocumentMut, String> {
        let path = config_path().ok_or("Cannot determine home directory")?;
        if !path.exists() {
            return Ok(DocumentMut::new());
        }
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        raw.parse::<DocumentMut>()
            .map_err(|e| format!("parse ~/.codex/config.toml: {}", e))
    }

    fn write_doc(doc: &DocumentMut) -> Result<(), String> {
        let path = config_path().ok_or("Cannot determine home directory")?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, doc.to_string()).map_err(|e| e.to_string())
    }

    pub fn list_installed() -> Result<Vec<ClaudePlugin>, String> {
        let doc = read_doc()?;
        let plugins_table = match doc.get("plugins").and_then(|i| i.as_table()) {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };
        let mut out: Vec<ClaudePlugin> = Vec::new();
        for (key, item) in plugins_table.iter() {
            // Each entry is a sub-table:  [plugins."name@marketplace"]
            let inner = match item.as_table() {
                Some(t) => t,
                None => continue,
            };
            let parts: Vec<&str> = key.splitn(2, '@').collect();
            let name = parts.first().unwrap_or(&"").to_string();
            let marketplace = parts.get(1).unwrap_or(&"").to_string();
            // Codex defaults to enabled=true when the field is omitted.
            let enabled = inner
                .get("enabled")
                .and_then(|i| i.as_value())
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let version = inner
                .get("version")
                .and_then(|i| i.as_value())
                .and_then(|v| v.as_str())
                .map(String::from);
            let install_path = inner
                .get("path")
                .and_then(|i| i.as_value())
                .and_then(|v| v.as_str())
                .map(String::from);
            out.push(ClaudePlugin {
                name,
                marketplace,
                enabled,
                version,
                install_path,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn set_enabled(plugin_key: &str, enabled: bool) -> Result<(), String> {
        let mut doc = read_doc()?;
        let plugins = doc
            .entry("plugins")
            .or_insert_with(|| Item::Table(Default::default()));
        let plugins_tbl = plugins
            .as_table_mut()
            .ok_or("config.toml's `plugins` exists but isn't a table")?;
        let entry = plugins_tbl
            .entry(plugin_key)
            .or_insert_with(|| Item::Table(Default::default()));
        let tbl = entry
            .as_table_mut()
            .ok_or_else(|| format!("[plugins.\"{}\"] is not a table", plugin_key))?;
        tbl["enabled"] = Item::Value(Value::from(enabled));
        write_doc(&doc)
    }

    pub fn uninstall(name: &str, marketplace: &str) -> Result<(), String> {
        // Removing the entry from config.toml drops it from Codex's
        // plugin list on next start. The on-disk plugin bundle is owned
        // by Codex; we don't touch it.
        let mut doc = read_doc()?;
        let plugins_tbl = match doc.get_mut("plugins").and_then(|i| i.as_table_mut()) {
            Some(t) => t,
            None => return Ok(()),
        };
        let key = format!("{}@{}", name, marketplace);
        plugins_tbl.remove(&key);
        write_doc(&doc)
    }
}
