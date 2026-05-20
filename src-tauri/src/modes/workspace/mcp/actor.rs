// Resolve the actor string used for attribution on every MCP-side
// mutation. Adding a new agent = one row in `KNOWN_AGENTS`; the
// returned string flows straight into `updated_by`, so the existing
// Inbox / attribution pipeline picks it up with zero extra wiring.

use serde_json::Value;

const KNOWN_AGENTS: &[(&str, &str)] = &[
    ("claude", "claude"),     // claude-code/<v>, anthropic-claude/<v>
    ("codex", "codex"),       // openai-codex / codex-cli
    ("gemini", "gemini"),     // google-gemini-cli
    ("opencode", "opencode"), // open-code project
    ("aider", "aider"),       // aider.chat
];

/// Map an incoming request to an actor string used for attribution.
///
/// Order of precedence:
///   1. `User-Agent` header — every CLI worth its salt sets one. We
///      lower-case it and look for a known agent slug. This handles
///      Claude Code, Codex, Gemini, OpenCode without per-agent setup.
///   2. `clientInfo.name` from the JSON-RPC `initialize` params (when
///      we can read it from the current request body) — second-best
///      because it's only present on init, but we accept any request
///      that happens to include it.
///   3. Default `'agent'` — generic fallback so attribution never
///      shows the literal string `'user'`.
pub(super) fn actor_from_request(headers: &axum::http::HeaderMap, body: &Value) -> String {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    if !ua.is_empty() {
        if let Some((_, actor)) = KNOWN_AGENTS.iter().find(|(slug, _)| ua.contains(slug)) {
            return (*actor).to_string();
        }
    }
    if let Some(name) = body
        .get("params")
        .and_then(|p| p.get("clientInfo"))
        .and_then(|c| c.get("name"))
        .and_then(|v| v.as_str())
    {
        let lower = name.to_lowercase();
        if let Some((_, actor)) = KNOWN_AGENTS.iter().find(|(slug, _)| lower.contains(slug)) {
            return (*actor).to_string();
        }
        // Sanitise unknown client name so it can't inject odd chars
        // into the DB. Keep alphanum + dashes.
        let cleaned: String = lower
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(32)
            .collect();
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    "agent".to_string()
}
