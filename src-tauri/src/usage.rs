use crate::models::*;

#[tauri::command]
pub async fn get_usage_analytics(days: Option<u32>) -> Result<UsageAnalytics, String> {
    tauri::async_runtime::spawn_blocking(move || get_usage_analytics_sync(days))
        .await
        .map_err(|e| format!("Thread error: {}", e))?
}

pub fn get_usage_analytics_sync(days: Option<u32>) -> Result<UsageAnalytics, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let projects_dir = home.join(".claude").join("projects");

    if !projects_dir.exists() {
        return Ok(UsageAnalytics {
            total_cost: 0.0, total_input_tokens: 0, total_output_tokens: 0,
            total_cache_read_tokens: 0, total_cache_write_tokens: 0,
            total_sessions: 0, total_api_calls: 0, cache_hit_percent: 0.0,
            daily: vec![], by_model: vec![], by_project: vec![],
            top_sessions: vec![], tools: vec![], shell_commands: vec![],
        });
    }

    let days_limit = days.unwrap_or(30);
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days_limit as i64);

    // Pricing per million tokens (approximate Claude pricing)
    let price_for_model = |model: &str| -> (f64, f64, f64, f64) {
        // (input, output, cache_read, cache_write) per million tokens
        let m = model.to_lowercase();
        if m.contains("opus") { (15.0, 75.0, 1.5, 18.75) }
        else if m.contains("haiku") { (0.80, 4.0, 0.08, 1.0) }
        else { (3.0, 15.0, 0.3, 3.75) } // sonnet default
    };

    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_write: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut total_calls: u32 = 0;
    let mut total_sessions: u32 = 0;

    let mut daily_map: std::collections::HashMap<String, (f64, u32, u64, u64)> = std::collections::HashMap::new();
    let mut model_map: std::collections::HashMap<String, (f64, u32, u64, u64, u64, u64)> = std::collections::HashMap::new();
    let mut project_map: std::collections::HashMap<String, (f64, u32, u32)> = std::collections::HashMap::new();
    let mut session_costs: Vec<SessionCost> = Vec::new();
    let mut tool_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut shell_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    // Iterate all project directories
    for project_entry in std::fs::read_dir(&projects_dir).map_err(|e| e.to_string())?.flatten() {
        let project_name = project_entry.file_name().to_string_lossy().to_string();
        let project_dir = project_entry.path();
        if !project_dir.is_dir() { continue; }

        let mut project_cost: f64 = 0.0;
        let mut project_sessions: u32 = 0;
        let mut project_calls: u32 = 0;

        // Iterate session files
        for session_entry in std::fs::read_dir(&project_dir).map_err(|e| e.to_string())?.flatten() {
            let path = session_entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }

            // Check modification time
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified_time < cutoff { continue; }
                }
            }

            let session_id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut session_cost: f64 = 0.0;
            let mut session_calls: u32 = 0;
            let mut session_model = String::new();
            total_sessions += 1;
            project_sessions += 1;

            for line in content.lines() {
                if line.trim().is_empty() { continue; }
                let val: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Extract model and usage from assistant messages
                let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if msg_type != "assistant" { continue; }

                let message = match val.get("message") {
                    Some(m) => m,
                    None => continue,
                };

                let model = message.get("model").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                if session_model.is_empty() { session_model = model.clone(); }

                let usage = match message.get("usage") {
                    Some(u) => u,
                    None => continue,
                };

                let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let output = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_read = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_write = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);

                let (pi, po, pcr, pcw) = price_for_model(&model);
                let call_cost = (input as f64 * pi + output as f64 * po + cache_read as f64 * pcr + cache_write as f64 * pcw) / 1_000_000.0;

                total_input += input;
                total_output += output;
                total_cache_read += cache_read;
                total_cache_write += cache_write;
                total_cost += call_cost;
                total_calls += 1;
                session_cost += call_cost;
                session_calls += 1;
                project_cost += call_cost;
                project_calls += 1;

                // Daily
                // Try to get timestamp from the entry
                let date_str = val.get("timestamp").and_then(|v| v.as_str())
                    .map(|t| t[..10].to_string())
                    .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
                let daily = daily_map.entry(date_str).or_insert((0.0, 0, 0, 0));
                daily.0 += call_cost;
                daily.1 += 1;
                daily.2 += input;
                daily.3 += output;

                // Model
                let short_model = if model.contains("opus") { "Opus".to_string() }
                    else if model.contains("haiku") { "Haiku".to_string() }
                    else if model.contains("sonnet") { "Sonnet".to_string() }
                    else { model.clone() };
                let me = model_map.entry(short_model).or_insert((0.0, 0, 0, 0, 0, 0));
                me.0 += call_cost;
                me.1 += 1;
                me.2 += input;
                me.3 += output;
                me.4 += cache_read;
                me.5 += cache_write;

                // Tools
                if let Some(content_arr) = message.get("content").and_then(|v| v.as_array()) {
                    for block in content_arr {
                        if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                            let tool_name = block.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            *tool_map.entry(tool_name.clone()).or_insert(0) += 1;

                            // Extract shell commands from Bash tool
                            if tool_name == "Bash" || tool_name == "bash" {
                                if let Some(input_obj) = block.get("input") {
                                    if let Some(cmd) = input_obj.get("command").and_then(|v| v.as_str()) {
                                        let shell_cmd = cmd.split_whitespace().next().unwrap_or("").to_string();
                                        if !shell_cmd.is_empty() {
                                            *shell_map.entry(shell_cmd).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if session_calls > 0 {
                session_costs.push(SessionCost {
                    session_id,
                    project: project_name.clone(),
                    cost: session_cost,
                    calls: session_calls,
                    model: session_model,
                });
            }
        }

        if project_sessions > 0 {
            project_map.insert(project_name, (project_cost, project_sessions, project_calls));
        }
    }

    // Sort and format results
    let mut daily: Vec<DailyUsage> = daily_map.into_iter().map(|(date, (cost, calls, input, output))| {
        DailyUsage { date, cost, calls, input_tokens: input, output_tokens: output }
    }).collect();
    daily.sort_by(|a, b| a.date.cmp(&b.date));

    let mut by_model: Vec<ModelUsage> = model_map.into_iter().map(|(model, (cost, calls, input, output, cr, cw))| {
        let total_input_for_model = input + cr + cw;
        let cache_pct = if total_input_for_model > 0 { (cr as f64 / total_input_for_model as f64) * 100.0 } else { 0.0 };
        ModelUsage { model, cost, calls, input_tokens: input, output_tokens: output, cache_hit_percent: cache_pct }
    }).collect();
    by_model.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    let mut by_project: Vec<ProjectUsage> = project_map.into_iter().map(|(project, (cost, sessions, calls))| {
        ProjectUsage { project, cost, sessions, calls }
    }).collect();
    by_project.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    session_costs.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));
    let top_sessions = session_costs.into_iter().take(5).collect();

    let mut tools: Vec<ToolCount> = tool_map.into_iter().map(|(name, count)| ToolCount { name, count }).collect();
    tools.sort_by(|a, b| b.count.cmp(&a.count));

    let mut shell_commands: Vec<ToolCount> = shell_map.into_iter().map(|(name, count)| ToolCount { name, count }).collect();
    shell_commands.sort_by(|a, b| b.count.cmp(&a.count));
    shell_commands.truncate(15);

    let total_all_input = total_input + total_cache_read + total_cache_write;
    let cache_hit_percent = if total_all_input > 0 { (total_cache_read as f64 / total_all_input as f64) * 100.0 } else { 0.0 };

    Ok(UsageAnalytics {
        total_cost,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_cache_read_tokens: total_cache_read,
        total_cache_write_tokens: total_cache_write,
        total_sessions,
        total_api_calls: total_calls,
        cache_hit_percent,
        daily,
        by_model,
        by_project,
        top_sessions,
        tools,
        shell_commands,
    })
}

/// Fetch usage limits via reqwest with native-tls (uses macOS SecureTransport to bypass Cloudflare)
#[tauri::command]
pub fn fetch_usage_limits(session_key: String) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .use_native_tls()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";
    let cookie = format!("sessionKey={}", session_key);

    // Step 1: Get org ID
    let orgs_resp = client
        .get("https://claude.ai/api/organizations")
        .header("Cookie", &cookie)
        .header("User-Agent", ua)
        .send()
        .map_err(|e| format!("orgs request failed: {}", e))?;

    let orgs: Vec<serde_json::Value> = orgs_resp
        .json()
        .map_err(|e| format!("orgs parse failed: {}", e))?;

    let org_id = orgs
        .first()
        .and_then(|o| o.get("uuid"))
        .and_then(|v| v.as_str())
        .ok_or("No organization found")?
        .to_string();

    // Step 2: Get usage
    let usage_resp = client
        .get(&format!("https://claude.ai/api/organizations/{}/usage", org_id))
        .header("Cookie", &cookie)
        .header("User-Agent", ua)
        .send()
        .map_err(|e| format!("usage request failed: {}", e))?;

    let usage: serde_json::Value = usage_resp
        .json()
        .map_err(|e| format!("usage parse failed: {}", e))?;

    Ok(usage)
}
