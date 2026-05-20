use super::types::{ChatContext, ChatMessage};

/// Safely truncate a string at a char boundary
pub fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    match s.char_indices().nth(max_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn format_context(ctx: &ChatContext) -> String {
    let mut parts = Vec::new();

    if let Some(req) = &ctx.current_request {
        let mut req_lines = vec![
            format!("  Method: {}", req.method),
            format!("  URL: {}", req.url),
        ];
        let enabled_headers: Vec<_> = req.headers.iter().filter(|h| h.enabled).collect();
        if !enabled_headers.is_empty() {
            let hdrs: Vec<String> = enabled_headers
                .iter()
                .map(|h| format!("{}: {}", h.key, h.value))
                .collect();
            req_lines.push(format!("  Headers: {}", hdrs.join(", ")));
        }
        let enabled_params: Vec<_> = req.params.iter().filter(|p| p.enabled).collect();
        if !enabled_params.is_empty() {
            let prms: Vec<String> = enabled_params
                .iter()
                .map(|p| format!("{}={}", p.key, p.value))
                .collect();
            req_lines.push(format!("  Params: {}", prms.join("&")));
        }
        if !req.body.is_empty() {
            let body_display = truncate_str(&req.body, 500);
            req_lines.push(format!("  Body ({}): {}", req.body_type, body_display));
        }
        if req.auth_type != "none" {
            req_lines.push(format!("  Auth: {}", req.auth_type));
        }
        parts.push(format!("Current Request:\n{}", req_lines.join("\n")));
    }

    if let Some(res) = &ctx.current_response {
        let body_display = truncate_str(&res.body, 500);
        parts.push(format!(
            "Current Response:\n  Status: {} {}\n  Duration: {}ms\n  Size: {} bytes\n  Body: {}",
            res.status, res.status_text, res.duration_ms, res.size_bytes, body_display
        ));
    }

    if !ctx.env_vars.is_empty() {
        let vars: Vec<String> = ctx
            .env_vars
            .iter()
            .map(|v| {
                if v.is_secret {
                    format!("{}=***", v.key)
                } else {
                    format!("{}={}", v.key, v.value)
                }
            })
            .collect();
        parts.push(format!("Environment Variables:\n  {}", vars.join(", ")));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("<context>\n{}\n</context>\n\n", parts.join("\n\n"))
    }
}

pub fn build_api_messages(
    messages: &[ChatMessage],
    context: &ChatContext,
) -> Vec<serde_json::Value> {
    let mut api_msgs = Vec::new();

    for msg in &messages[..messages.len().saturating_sub(1)] {
        api_msgs.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    if let Some(last) = messages.last() {
        let context_str = format_context(context);
        let content = if context_str.is_empty() {
            last.content.clone()
        } else {
            format!("{}{}", context_str, last.content)
        };
        api_msgs.push(serde_json::json!({
            "role": "user",
            "content": content,
        }));
    }

    api_msgs
}
