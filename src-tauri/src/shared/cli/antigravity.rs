// Antigravity CLI implementation of [`CliRunner`].
//
// `agy` is Google's AI coding agent CLI. Binary name is `agy`; home
// directory is `~/.antigravity`.

use std::path::{Path, PathBuf};

use super::runner::{CliRunner, SpawnOpts};
use crate::shared::platform::shell::default_user_shell;

pub struct AntigravityRunner;

const BINARY: &str = "agy";
const HOME_SUBDIR: &str = ".antigravity";
const PLUGINS_SUBDIR: &str = "extensions";
const SESSIONS_SUBDIR: &str = "antigravity";
const SESSION_EXT: &str = "jsonl";

impl AntigravityRunner {
    fn dot_antigravity(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(HOME_SUBDIR))
    }
}

impl CliRunner for AntigravityRunner {
    fn id(&self) -> &'static str {
        "antigravity"
    }

    fn binary_name(&self) -> &'static str {
        BINARY
    }

    fn resolve_binary_path(&self) -> String {
        crate::shared::platform::path::find_binary(BINARY)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| BINARY.to_string())
    }

    fn build_spawn_command(&self, opts: &SpawnOpts) -> String {
        let head = opts.binary_path_override.as_deref()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(crate::shared::cli::runner::shell_quote_path)
            .unwrap_or_else(|| BINARY.to_string());
        let mut cmd = head;

        if let Some(ref sid) = opts.resume_session_id {
            cmd.push_str(&format!(" --conversation \"{}\"", sid));
        } else {
            // --prompt-interactive takes the initial prompt as a value.
            cmd.push_str(" --prompt-interactive");
            let initial = opts.system_prompt.as_deref().filter(|s| !s.trim().is_empty())
                .unwrap_or("Let's start working");
            cmd.push_str(&format!(" {}", crate::shared::cli::runner::shell_quote_path(initial)));
        }

        if opts.skip_permissions {
            cmd.push_str(" --dangerously-skip-permissions");
        }

        cmd
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.dot_antigravity()
    }

    fn plugins_dir(&self) -> Option<PathBuf> {
        self.dot_antigravity().map(|p| p.join(PLUGINS_SUBDIR))
    }

    fn settings_file(&self) -> Option<PathBuf> {
        self.dot_antigravity().map(|p| p.join("argv.json"))
    }

    fn installed_plugins_file(&self) -> Option<PathBuf> {
        self.plugins_dir()
    }

    fn plugin_marketplaces_dir(&self) -> Option<PathBuf> {
        None // agy does not have marketplace subdirectories
    }

    fn plugin_install_counts_file(&self) -> Option<PathBuf> {
        None
    }

    fn run_plugin_subcommand(&self, args: &[&str]) -> Result<(bool, String), String> {
        let mut parts: Vec<&str> = vec![BINARY, "plugin"];
        parts.extend_from_slice(args);
        let cmd = parts.join(" ");

        let (shell_path, shell_kind) = default_user_shell();
        let shell_args = shell_kind.exec_command_argv(&cmd);

        let output = std::process::Command::new(&shell_path)
            .args(&shell_args)
            .output()
            .map_err(|e| format!("Failed to run plugin subcommand: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() {
            let msg = if stderr.is_empty() { stdout } else { stderr };
            return Ok((false, msg));
        }
        Ok((true, stdout))
    }

    fn sessions_root(&self) -> Option<PathBuf> {
        self.dot_antigravity().map(|p| p.join(SESSIONS_SUBDIR))
    }

    fn session_dir_for_project(&self, project_path: &str) -> Option<PathBuf> {
        // Use same encoding as Claude: flatten special chars to `-`.
        let encoded: String = project_path
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        self.sessions_root().map(|r| r.join(encoded))
    }

    fn session_file_extension(&self) -> &'static str {
        SESSION_EXT
    }

    fn extract_resume_id_from_output(&self, buffer: &str) -> Option<String> {
        // agy outputs "agy --conversation <id>" for resume.
        let needle = "agy --conversation ";
        let start = buffer.find(needle)? + needle.len();
        let rest = &buffer[start..];
        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
            .collect();
        if id.is_empty() { None } else { Some(id) }
    }

    fn usage_api_orgs_url(&self) -> Option<String> {
        None
    }

    fn usage_api_url_for(&self, _org_id: &str) -> Option<String> {
        None
    }

    fn is_session_file(&self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()) == Some(SESSION_EXT)
    }
}

/// Process-wide stateless instance.
pub static ANTIGRAVITY: AntigravityRunner = AntigravityRunner;
