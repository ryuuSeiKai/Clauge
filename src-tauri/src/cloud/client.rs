// Typed HTTP client for the Clauge Worker API. Uses the shared proxy-aware
// reqwest client built by `shared::http`.

use serde::de::DeserializeOwned;
use sqlx::SqlitePool;

use crate::cloud::auth::AuthState;
use crate::cloud::config::API_BASE_URL;
use crate::cloud::models::{
    AuthResponse, MeResponse, SyncPullResponse, SyncPushResponse, SyncStateRow,
};
use crate::shared::http::build_app_http_client;

/// Errors all callers see — unified shape so commands can map to user-friendly messages.
#[derive(Debug)]
pub enum CloudError {
    NotAuthenticated,
    Network(String),
    Server { status: u16, body: String },
    /// 412 Precondition Failed from `/api/sync/push/:kind` — remote hash
    /// doesn't match the `prevHash` we sent. Caller decides whether to
    /// pull, prompt the user, or force-push.
    Conflict { current_hash: Option<String>, current_updated_at: Option<String> },
}

impl std::fmt::Display for CloudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudError::NotAuthenticated => write!(f, "Not signed in to Clauge cloud"),
            CloudError::Network(e) => write!(f, "Network error: {}", e),
            CloudError::Server { status, body } => write!(f, "Cloud API {}: {}", status, body),
            CloudError::Conflict { .. } => write!(f, "Remote has changed since this device last synced."),
        }
    }
}

impl From<CloudError> for String {
    fn from(e: CloudError) -> String {
        e.to_string()
    }
}

// ─── Exchange endpoints (no auth required) ──────────────────────────────────

pub async fn exchange_github(pool: &SqlitePool, code: &str) -> Result<AuthResponse, CloudError> {
    post_json_no_auth(
        pool,
        "/api/auth/github/exchange",
        serde_json::json!({ "code": code }),
    )
    .await
}

pub async fn exchange_google(
    pool: &SqlitePool,
    code: &str,
    redirect_uri: &str,
) -> Result<AuthResponse, CloudError> {
    post_json_no_auth(
        pool,
        "/api/auth/google/exchange",
        serde_json::json!({ "code": code, "redirectUri": redirect_uri }),
    )
    .await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleRefreshResponse {
    pub token: String,
    pub id_token: Option<String>,
    pub expires_in: Option<i64>,
}

pub async fn refresh_google(
    pool: &SqlitePool,
    refresh_token: &str,
) -> Result<GoogleRefreshResponse, CloudError> {
    post_json_no_auth(
        pool,
        "/api/auth/google/refresh",
        serde_json::json!({ "refreshToken": refresh_token }),
    )
    .await
}

// ─── Auth-required endpoints ────────────────────────────────────────────────

pub async fn me(pool: &SqlitePool, state: &AuthState) -> Result<MeResponse, CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    get_json(pool, "/api/auth/me", &token, &provider).await
}

pub async fn update_profile(
    pool: &SqlitePool,
    state: &AuthState,
    display_name: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
) -> Result<MeResponse, CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let url = format!("{}{}", API_BASE_URL, "/api/auth/me");
    // Only include fields the caller passed — `null` clears, missing = leave alone.
    let mut body = serde_json::Map::new();
    if let Some(v) = display_name { body.insert("displayName".into(), serde_json::Value::String(v)); }
    if let Some(v) = first_name   { body.insert("firstName".into(),   serde_json::Value::String(v)); }
    if let Some(v) = last_name    { body.insert("lastName".into(),    serde_json::Value::String(v)); }
    let resp = client
        .patch(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .header("Content-Type", "application/json")
        .json(&serde_json::Value::Object(body))
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await
}

pub async fn delete_account(
    pool: &SqlitePool,
    state: &AuthState,
    confirm_slug: &str,
) -> Result<(), CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let url = format!("{}{}", API_BASE_URL, "/api/auth/me");
    let resp = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .header("X-Confirm", confirm_slug)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await.map(|_: serde_json::Value| ())
}

pub async fn link(
    pool: &SqlitePool,
    state: &AuthState,
    provider: &str,
    code: &str,
    redirect_uri: Option<&str>,
) -> Result<MeResponse, CloudError> {
    let (token, active_provider) = state
        .active_token_and_provider()
        .ok_or(CloudError::NotAuthenticated)?;
    let body = serde_json::json!({
        "provider": provider,
        "code": code,
        "redirectUri": redirect_uri.unwrap_or("https://clauge.in/auth/google-callback.html"),
    });
    post_json_auth(pool, "/api/auth/link", body, &token, &active_provider).await
}

pub async fn unlink(
    pool: &SqlitePool,
    state: &AuthState,
    provider: &str,
) -> Result<MeResponse, CloudError> {
    let (token, active_provider) = state
        .active_token_and_provider()
        .ok_or(CloudError::NotAuthenticated)?;
    post_json_auth(
        pool,
        "/api/auth/unlink",
        serde_json::json!({ "provider": provider }),
        &token,
        &active_provider,
    )
    .await
}

// ─── Sync endpoints ─────────────────────────────────────────────────────────

pub async fn sync_state(
    pool: &SqlitePool,
    state: &AuthState,
) -> Result<Vec<SyncStateRow>, CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    get_json(pool, "/api/sync/state", &token, &provider).await
}

pub async fn sync_pull(
    pool: &SqlitePool,
    state: &AuthState,
    kind: &str,
) -> Result<SyncPullResponse, CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    let path = format!("/api/sync/pull/{}", kind);
    get_json(pool, &path, &token, &provider).await
}

/// Push a kind blob with optimistic concurrency.
///
/// `prev_hash` semantics (matches the Worker side):
///   - `None`     → first push of this kind (server requires row to not exist).
///   - `Some("*")` → force overwrite (post-conflict "Keep my changes").
///   - `Some(hex)` → only succeed if remote currently has that hash.
///
/// On 412 the server returns the current remote state; we surface it as
/// `CloudError::Conflict` so the orchestrator can flip the kind into
/// conflict-locked mode and the UI can show the resolver.
pub async fn sync_push(
    pool: &SqlitePool,
    state: &AuthState,
    kind: &str,
    content_hash: &str,
    payload_b64: &str,
    prev_hash: Option<&str>,
) -> Result<SyncPushResponse, CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let url = format!("{}{}/{}", API_BASE_URL, "/api/sync/push", kind);

    let mut body = serde_json::Map::new();
    body.insert("contentHash".into(), serde_json::Value::String(content_hash.into()));
    body.insert("payload".into(), serde_json::Value::String(payload_b64.into()));
    if let Some(p) = prev_hash {
        body.insert("prevHash".into(), serde_json::Value::String(p.into()));
    }

    let resp = client
        .put(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .header("Content-Type", "application/json")
        .json(&serde_json::Value::Object(body))
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;

    // 412 → conflict. Parse the body to surface the current remote hash so
    // the resolver can show "this device vs other device" stats.
    if resp.status().as_u16() == 412 {
        #[derive(serde::Deserialize)]
        struct ConflictBody {
            #[serde(rename = "currentHash")] current_hash: Option<String>,
            #[serde(rename = "currentUpdatedAt")] current_updated_at: Option<String>,
        }
        let body: ConflictBody = resp.json().await.unwrap_or(ConflictBody {
            current_hash: None,
            current_updated_at: None,
        });
        return Err(CloudError::Conflict {
            current_hash: body.current_hash,
            current_updated_at: body.current_updated_at,
        });
    }

    check_ok(resp).await
}

pub async fn sync_wipe(pool: &SqlitePool, state: &AuthState) -> Result<(), CloudError> {
    let (token, provider) = state.active_token_and_provider().ok_or(CloudError::NotAuthenticated)?;
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let url = format!("{}{}", API_BASE_URL, "/api/sync/wipe");
    let resp = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .header("X-Confirm", "yes")
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await.map(|_: serde_json::Value| ())
}

// ─── Internals ──────────────────────────────────────────────────────────────

async fn post_json_no_auth<T: DeserializeOwned>(
    pool: &SqlitePool,
    path: &str,
    body: serde_json::Value,
) -> Result<T, CloudError> {
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let resp = client
        .post(format!("{}{}", API_BASE_URL, path))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await
}

async fn post_json_auth<T: DeserializeOwned>(
    pool: &SqlitePool,
    path: &str,
    body: serde_json::Value,
    token: &str,
    provider: &str,
) -> Result<T, CloudError> {
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let resp = client
        .post(format!("{}{}", API_BASE_URL, path))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await
}

async fn get_json<T: DeserializeOwned>(
    pool: &SqlitePool,
    path: &str,
    token: &str,
    provider: &str,
) -> Result<T, CloudError> {
    let client = build_app_http_client(pool).await.map_err(CloudError::Network)?;
    let resp = client
        .get(format!("{}{}", API_BASE_URL, path))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Provider", provider)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    check_ok(resp).await
}

async fn check_ok<T: DeserializeOwned>(
    resp: tauri_plugin_http::reqwest::Response,
) -> Result<T, CloudError> {
    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(CloudError::Server { status, body });
    }
    resp.json::<T>().await.map_err(|e| CloudError::Network(e.to_string()))
}
