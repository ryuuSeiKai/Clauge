// Tauri commands exposed to the frontend. Thin wrappers around the `cloud::*`
// internals; bulk of the logic lives in `auth.rs`, `client.rs`, `sync.rs`.

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::cloud::auth::{self, AuthState};
use crate::cloud::client::{self, CloudError};
use crate::cloud::config::{self as cloud_config, api_base_url, settings_key_synced_at, SETTINGS_KEY_HAS_SYNCED};
use crate::cloud::domains::ALL_KINDS;
use crate::cloud::models::{CloudAiBalance, CloudAiUsage, CloudPricing, CloudStatus, CloudUser};
use crate::cloud::pro_state::ProStateManager;
use crate::cloud::scheduler::Scheduler;
use crate::cloud::sync;
use crate::cloud::{ai as ai_client, billing as billing_client};
use crate::shared::repos::settings;

// ─── Status / OAuth URL builders ────────────────────────────────────────────

#[tauri::command]
pub async fn cloud_get_status(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
) -> Result<CloudStatus, String> {
    let snap = state.snapshot();
    if !state.is_connected() {
        return Ok(CloudStatus::default());
    }

    // Fetch fresh `me` from the server. The client::me path now
    // self-refreshes on 401 (Google only — see `with_google_refresh_retry`),
    // so by the time we get here:
    //   • Ok       → token was either valid or successfully refreshed.
    //                Route the response through ProStateManager — that
    //                runs the Pro↔Free transition hooks (coworker
    //                disable/enable), persists the snapshot, and emits
    //                cloud:pro-state for the frontend.
    //   • NotAuth  → refresh exhausted / no refresh path; ask the manager
    //                to clear (runs the Pro→Free hook based on in-memory
    //                state, immune to the SQLite-key-already-deleted race)
    //                then wipe auth tokens.
    //   • Other    → network / 5xx; keep partial state, leave the manager
    //                untouched so a transient outage doesn't fake a
    //                downgrade.
    match client::me(pool.inner(), &state).await {
        Ok(me) => {
            let mut last_synced = std::collections::HashMap::new();
            for k in ALL_KINDS {
                if let Ok(Some(s)) = settings::get_by_key(pool.inner(), &settings_key_synced_at(k)).await {
                    last_synced.insert(k.to_string(), s.value);
                }
            }

            pro_state
                .apply_from_entitlements(&me.entitlements, Some(&me.plan), &app, pool.inner())
                .await?;

            Ok(CloudStatus {
                connected: true,
                active_provider: snap.active_provider,
                user: Some(me.user),
                providers: me.providers,
                plan: me.plan,
                last_synced,
                entitlements: Some(me.entitlements),
            })
        }
        Err(CloudError::NotAuthenticated) => {
            // Manager.clear FIRST so the downgrade hook reads in-memory
            // Pro state (still valid) and runs disable_beyond_first_n
            // BEFORE auth::clear wipes the SQLite key it used to depend
            // on. Then auth::clear tears down tokens + settings.
            let _ = pro_state.clear(&app, pool.inner()).await;
            let _ = auth::clear(&state, pool.inner()).await;
            Ok(CloudStatus::default())
        }
        Err(_) => Ok(CloudStatus {
            connected: true,
            active_provider: snap.active_provider,
            user: snap.user_id.map(|id| CloudUser {
                user_id: id,
                email: None,
                display_name: None,
                first_name: None,
                last_name: None,
                avatar_url: None,
                slug: String::new(),
                created_at: None,
            }),
            providers: Vec::new(),
            plan: "free".into(),
            last_synced: Default::default(),
            entitlements: None,
        }),
    }
}

#[tauri::command]
pub fn cloud_github_login_url(state: String) -> String {
    auth::github_oauth_url(&state)
}

#[tauri::command]
pub fn cloud_google_login_url(state: String) -> String {
    auth::google_oauth_url(&state)
}

#[tauri::command]
pub async fn cloud_create_ticket(pool: State<'_, SqlitePool>) -> Result<String, String> {
    let client = crate::shared::http::build_app_http_client(pool.inner())
        .await
        .map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .post(format!("{}/api/auth/ticket", api_base_url()))
        .send()
        .await
        .map_err(|e| format!("create ticket: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse ticket: {}", e))?;
    body["ticket"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "missing ticket id".to_string())
}

/// Response shape from GET /api/auth/ticket/{id}.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TicketPollResult {
    pub status: String,
    pub token: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
}

#[tauri::command]
pub async fn cloud_poll_ticket(
    pool: State<'_, SqlitePool>,
    ticket: String,
) -> Result<TicketPollResult, String> {
    let client = crate::shared::http::build_app_http_client(pool.inner())
        .await
        .map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(format!("{}/api/auth/ticket/{}", api_base_url(), ticket))
        .send()
        .await
        .map_err(|e| format!("poll ticket: {}", e))?;
    let body: TicketPollResult = resp
        .json()
        .await
        .map_err(|e| format!("parse ticket response: {}", e))?;
    Ok(body)
}

// ─── API URL override (self-hosted server / zrok tunnel) ────────────────────

#[tauri::command]
pub async fn cloud_set_api_url(
    pool: State<'_, SqlitePool>,
    url: Option<String>,
) -> Result<(), String> {
    // Persist to settings DB so it survives restart.
    if let Some(ref u) = url {
        settings::upsert(pool.inner(), crate::cloud::config::SETTINGS_KEY_API_URL, u)
            .await
            .map_err(|e| format!("save api url: {}", e))?;
    } else {
        // Clear — delete the setting row.
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(crate::cloud::config::SETTINGS_KEY_API_URL)
            .execute(pool.inner())
            .await
            .map_err(|e| format!("clear api url: {}", e))?;
    }
    crate::cloud::config::set_api_url_override(url);
    log::info!("[cloud] API URL override updated");
    Ok(())
}

#[tauri::command]
pub async fn cloud_get_api_url() -> String {
    crate::cloud::config::api_base_url()
}

// ─── OAuth code exchange (deep-link → here) ─────────────────────────────────

#[tauri::command]
pub async fn cloud_exchange_code(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
    provider: String,
    code: String,
) -> Result<CloudStatus, String> {
    match provider.as_str() {
        "github" => {
            let resp = client::exchange_github(pool.inner(), &code).await?;
            let token = resp.token.clone().ok_or_else(|| "missing token".to_string())?;
            auth::store_github(&state, pool.inner(), &token, resp.user.user_id).await?;
            after_login(&app, pool.inner(), &state).await?;
            // Eagerly populate manager from the exchange response so the
            // Free→Pro hook fires immediately (no wait for the first
            // cloud_get_status round trip).
            pro_state
                .apply_from_entitlements(&resp.entitlements, Some(&resp.plan), &app, pool.inner())
                .await?;
            Ok(build_status(pool.inner(), &state, &resp).await)
        }
        "google" => {
            let resp = client::exchange_google(
                pool.inner(),
                &code,
                &format!("{}/auth/google-callback.html", crate::cloud::config::api_base_url()),
            )
            .await?;
            let id_token = resp.id_token.clone().ok_or_else(|| "missing id_token".to_string())?;
            auth::store_google(
                &state,
                pool.inner(),
                resp.token.as_deref(),
                resp.refresh.as_deref(),
                &id_token,
                resp.user.user_id,
            )
            .await?;
            after_login(&app, pool.inner(), &state).await?;
            pro_state
                .apply_from_entitlements(&resp.entitlements, Some(&resp.plan), &app, pool.inner())
                .await?;
            Ok(build_status(pool.inner(), &state, &resp).await)
        }
        _ => Err(format!("unknown provider: {}", provider)),
    }
}

// ─── Linking ───────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cloud_link_provider(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
    provider: String,
    code: String,
) -> Result<CloudStatus, String> {
    let me = client::link(pool.inner(), &state, &provider, &code, None)
        .await
        .map_err(String::from)?;
    pro_state
        .apply_from_entitlements(&me.entitlements, Some(&me.plan), &app, pool.inner())
        .await?;
    let snap = state.snapshot();
    Ok(CloudStatus {
        connected: true,
        active_provider: snap.active_provider,
        user: Some(me.user),
        providers: me.providers,
        plan: me.plan,
        last_synced: Default::default(),
        entitlements: Some(me.entitlements),
    })
}

#[tauri::command]
pub async fn cloud_update_profile(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
    display_name: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
) -> Result<CloudStatus, String> {
    let me = client::update_profile(pool.inner(), &state, display_name, first_name, last_name)
        .await
        .map_err(String::from)?;
    pro_state
        .apply_from_entitlements(&me.entitlements, Some(&me.plan), &app, pool.inner())
        .await?;
    let snap = state.snapshot();
    Ok(CloudStatus {
        connected: true,
        active_provider: snap.active_provider,
        user: Some(me.user),
        providers: me.providers,
        plan: me.plan,
        last_synced: Default::default(),
        entitlements: Some(me.entitlements),
    })
}

#[tauri::command]
pub async fn cloud_unlink_provider(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
    provider: String,
) -> Result<CloudStatus, String> {
    let me = client::unlink(pool.inner(), &state, &provider)
        .await
        .map_err(String::from)?;
    pro_state
        .apply_from_entitlements(&me.entitlements, Some(&me.plan), &app, pool.inner())
        .await?;
    let snap = state.snapshot();
    Ok(CloudStatus {
        connected: true,
        active_provider: snap.active_provider,
        user: Some(me.user),
        providers: me.providers,
        plan: me.plan,
        last_synced: Default::default(),
        entitlements: Some(me.entitlements),
    })
}

// ─── Sync surface ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn cloud_check_remote_exists(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<bool, String> {
    let rows = client::sync_state(pool.inner(), &state)
        .await
        .map_err(String::from)?;
    Ok(!rows.is_empty())
}

#[tauri::command]
pub async fn cloud_sync_push_now(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<Vec<String>, String> {
    let kinds: Vec<&str> = ALL_KINDS.iter().copied().collect();
    sync::push_all(pool.inner(), &state, &kinds).await
}

/// List kinds currently in conflict-locked state. Used by the resolver
/// UI to render the amber dot, the "Action Required (N)" label, and the
/// modal body.
#[tauri::command]
pub async fn cloud_get_conflicts(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<String>, String> {
    sync::conflicted_kinds(pool.inner()).await
}

/// Resolve all conflicts by force-pushing this device's data — the user
/// has picked "Keep my changes" in the resolver modal. Iterates all
/// conflicted kinds; any individual failure short-circuits.
#[tauri::command]
pub async fn cloud_resolve_keep_local(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<(), String> {
    let kinds = sync::conflicted_kinds(pool.inner()).await?;
    for k in &kinds {
        sync::force_push_kind(pool.inner(), &state, k).await?;
    }
    Ok(())
}

/// Resolve all conflicts by adopting the remote — the user has picked
/// "Use other device's" in the resolver. Pulls each conflicted kind and
/// clears its conflict flag.
#[tauri::command]
pub async fn cloud_resolve_use_remote(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<(), String> {
    let kinds = sync::conflicted_kinds(pool.inner()).await?;
    for k in &kinds {
        sync::resolve_use_remote(pool.inner(), &state, k).await?;
    }
    Ok(())
}

/// Lightweight remote-state check used by pull-on-focus: returns the
/// kinds where the server has moved past our last-known synced hash AND
/// local has no unpushed changes (safe to silently pull). Caller pulls
/// those, then re-emits cloud:synced for the frontend to refresh stamps.
#[tauri::command]
pub async fn cloud_pull_if_remote_newer(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<Vec<String>, String> {
    sync::pull_if_remote_newer(pool.inner(), &state).await
}

#[tauri::command]
pub async fn cloud_sync_restore(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<Vec<String>, String> {
    let pulled = sync::pull_all(pool.inner(), &state).await?;
    settings::upsert(pool.inner(), SETTINGS_KEY_HAS_SYNCED, "true")
        .await
        .map_err(|e| format!("mark synced: {}", e))?;
    Ok(pulled)
}

#[tauri::command]
pub async fn cloud_local_has_data(pool: State<'_, SqlitePool>) -> Result<bool, String> {
    sync::local_has_data(pool.inner()).await
}

// ─── Account management ───────────────────────────────────────────────────

#[tauri::command]
pub async fn cloud_logout(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
) -> Result<(), String> {
    if let Some(s) = app.try_state::<Scheduler>() {
        s.disable_and_clear();
    }
    // Manager.clear runs the Pro→Free downgrade hook (soft-disabling
    // coworkers 4+) BEFORE auth::clear wipes the SQLite cloud:* keys.
    // Closes the bug where signing out left all coworkers visibly active
    // because the inline transition guard in cloud_get_status keyed off
    // a cloud:plan SQLite row that auth::clear had just deleted.
    let _ = pro_state.clear(&app, pool.inner()).await;
    auth::clear(&state, pool.inner()).await
}

#[tauri::command]
pub async fn cloud_wipe_remote(
    _app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    _pro_state: State<'_, ProStateManager>,
) -> Result<(), String> {
    client::sync_wipe(pool.inner(), &state)
        .await
        .map_err(String::from)?;

    // Clear local sync bookkeeping AFTER the wipe succeeds. Without this,
    // the next 5s-debounced auto-push would compare its content_hash
    // against `cloud:hash:<kind>` (still matching the pre-wipe state),
    // see "no change", and skip the upload — leaving the cloud empty
    // forever until the user makes a local change. Clearing these rows
    // forces the next push to actually run.
    //
    // We DO NOT call `cloud_logout` here (previous behaviour). That
    // contradicted the UI promise of "your account stays — you can
    // re-push anytime" and forced the user to sign back in. After this
    // change: account stays signed in, cloud is empty, any local
    // mutation re-pushes fresh on the next auto-push tick.
    let _ = sqlx::query(
        "DELETE FROM settings WHERE key LIKE 'cloud:hash:%' OR key LIKE 'cloud:synced_at:%'",
    )
    .execute(pool.inner())
    .await;

    Ok(())
}

#[tauri::command]
pub async fn cloud_delete_account(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
    confirmation_slug: String,
) -> Result<(), String> {
    client::delete_account(pool.inner(), &state, &confirmation_slug)
        .await
        .map_err(String::from)?;
    cloud_logout(app.clone(), pool, state, pro_state).await?;
    let _ = app.emit_to("main", "cloud:account-deleted", ());
    Ok(())
}

// ─── Helpers ───────────────────────────────────────────────────────────────

/// After a successful login: enable the scheduler so subsequent mutations bump.
async fn after_login(
    app: &AppHandle,
    _pool: &SqlitePool,
    _state: &AuthState,
) -> Result<(), String> {
    if let Some(s) = app.try_state::<Scheduler>() {
        s.enable();
    }
    Ok(())
}

async fn build_status(
    pool: &SqlitePool,
    state: &AuthState,
    resp: &crate::cloud::models::AuthResponse,
) -> CloudStatus {
    let snap = state.snapshot();
    let mut last_synced = std::collections::HashMap::new();
    for k in ALL_KINDS {
        if let Ok(Some(s)) = settings::get_by_key(pool, &settings_key_synced_at(k)).await {
            last_synced.insert(k.to_string(), s.value);
        }
    }
    CloudStatus {
        connected: true,
        active_provider: snap.active_provider,
        user: Some(resp.user.clone()),
        providers: resp.providers.clone(),
        plan: resp.plan.clone(),
        last_synced,
        entitlements: Some(resp.entitlements.clone()),
    }
}

// ─── cloud billing + AI proxy wrappers ──────────────────────────────────────

#[tauri::command]
pub async fn cloud_get_pricing(
    pool: State<'_, SqlitePool>,
) -> Result<CloudPricing, String> {
    billing_client::get_pricing(pool.inner()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cloud_create_checkout(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    plan: String,
) -> Result<String, String> {
    let (token, provider) = state
        .active_token_and_provider()
        .ok_or_else(|| "not signed in".to_string())?;
    let resp = billing_client::create_checkout(pool.inner(), &token, &provider, &plan)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.url)
}

#[tauri::command]
pub async fn cloud_open_portal(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
) -> Result<String, String> {
    let (token, provider) = state
        .active_token_and_provider()
        .ok_or_else(|| "not signed in".to_string())?;
    let resp = billing_client::create_portal_session(pool.inner(), &token, &provider)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp.url)
}

#[tauri::command]
pub async fn cloud_ai_balance(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    pro_state: State<'_, ProStateManager>,
) -> Result<CloudAiBalance, String> {
    let (token, provider) = state
        .active_token_and_provider()
        .ok_or_else(|| "not signed in".to_string())?;
    let balance = ai_client::get_balance(pool.inner(), &token, &provider)
        .await
        .map_err(|e| e.to_string())?;
    // Patch the manager so the cloud:pro-state event fires and derived
    // cloudCredits in the frontend updates. AccountTab's refreshBalance
    // (and any other future caller) no longer needs to set the store.
    let _ = pro_state
        .patch_credits_remaining(balance.remaining, &app, pool.inner())
        .await;
    Ok(balance)
}

#[tauri::command]
pub async fn cloud_ai_usage(
    pool: State<'_, SqlitePool>,
    state: State<'_, AuthState>,
    limit: Option<u32>,
    before: Option<String>,
) -> Result<CloudAiUsage, String> {
    let (token, provider) = state
        .active_token_and_provider()
        .ok_or_else(|| "not signed in".to_string())?;
    ai_client::get_usage(pool.inner(), &token, &provider, limit, before.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Returns the active cloud bearer token + provider slug for the JS layer
/// to use when invoking ai_chat with provider = "Synape". Returns None if
/// the user isn't signed in.
#[tauri::command]
pub fn cloud_get_active_token(
    state: State<'_, AuthState>,
) -> Option<(String, String)> {
    state.active_token_and_provider()
}

use std::fs;
use std::path::PathBuf;

fn skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".antigravity").join("skills"))
}

#[derive(serde::Serialize)]
pub struct InstalledSkill {
    pub name: String,
    pub path: String,
    pub size: u64,
}

#[tauri::command]
pub fn cloud_install_skill(name: String, content: String) -> Result<(), String> {
    let dir = skills_dir().ok_or("Cannot determine home directory")?;
    fs::create_dir_all(&dir).map_err(|e| format!("create skills dir: {}", e))?;
    let path = dir.join(format!("{}.md", name));
    fs::write(&path, &content).map_err(|e| format!("write skill: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn cloud_uninstall_skill(name: String) -> Result<(), String> {
    let dir = skills_dir().ok_or("Cannot determine home directory")?;
    let path = dir.join(format!("{}.md", name));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove skill: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub fn cloud_list_installed_skills() -> Result<Vec<InstalledSkill>, String> {
    let dir = match skills_dir() {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut skills = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            skills.push(InstalledSkill {
                name,
                path: path.to_string_lossy().to_string(),
                size,
            });
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

