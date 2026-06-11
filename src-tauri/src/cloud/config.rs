use std::sync::Mutex;

/// Runtime override set from Tauri command or settings DB. Checked first
/// by `api_base_url()`, before env vars and compile-time fallback.
static API_URL_OVERRIDE: Mutex<Option<String>> = Mutex::new(None);

/// Override the API base URL at runtime (e.g. from a zrok tunnel).
/// Pass `None` to clear the override.
pub fn set_api_url_override(url: Option<String>) {
    *API_URL_OVERRIDE.lock().unwrap() = url;
}

/// Returns the current effective API base URL:
///   1. Runtime override (set via Tauri command or loaded from settings DB)
///   2. `SYNAPE_API_URL` env var (runtime)
///   3. `Synape_API_URL` env var (compile-time compat)
///   4. Compile-time default `http://67.217.243.181:3000`
pub fn api_base_url() -> String {
    // 1. Runtime override
    if let Some(url) = API_URL_OVERRIDE.lock().unwrap().as_ref() {
        return url.clone();
    }
    // 2. Runtime env vars
    if let Ok(url) = std::env::var("SYNAPE_API_URL") {
        return url;
    }
    if let Ok(url) = std::env::var("Synape_API_URL") {
        return url;
    }
    // 3. Compile-time fallback
    option_env!("Synape_API_URL").unwrap_or("http://67.217.243.181:3000").to_string()
}

// OS keyring service for cloud auth tokens. Separate service from SSH /
// Explorer secrets so wiping cloud auth doesn't touch user-saved credentials.
pub const KEYRING_SERVICE: &str = "Synape Cloud Auth";

pub const KEY_GITHUB_ACCESS: &str = "github:access_token";
pub const KEY_GOOGLE_ACCESS: &str = "google:access_token";
pub const KEY_GOOGLE_REFRESH: &str = "google:refresh_token";
pub const KEY_GOOGLE_ID: &str = "google:id_token";

// settings table keys for non-secret bookkeeping.
pub const SETTINGS_KEY_USER_ID: &str = "cloud:user_id";
pub const SETTINGS_KEY_ACTIVE_PROVIDER: &str = "cloud:active_provider";
pub const SETTINGS_KEY_HAS_SYNCED: &str = "cloud:has_synced";
pub const SETTINGS_KEY_PLAN: &str = "cloud:plan";
// JSON snapshots of entitlements — read at boot so the AccountTab and
// pro-gates render the last-known values before the /api/auth/me round
// trip lands. Wiped by `auth::clear()` along with all other `cloud:%` rows.
pub const SETTINGS_KEY_CREDITS_SNAPSHOT: &str = "cloud:credits_snapshot";
pub const SETTINGS_KEY_SUB_SNAPSHOT: &str = "cloud:sub_snapshot";

/// Override for the API base URL (set from frontend settings, persisted
/// in the `settings` table). NOT wiped by `auth::clear()` — survives
/// sign-out so the user doesn't lose their tunnel URL on reconnect.
pub const SETTINGS_KEY_API_URL: &str = "cloud:api_url";

// Per-kind last-pushed hash; key format `cloud:hash:<kind>`.
pub fn settings_key_hash(kind: &str) -> String {
    format!("cloud:hash:{}", kind)
}
// Per-kind last-synced ISO timestamp; key format `cloud:synced_at:<kind>`.
pub fn settings_key_synced_at(kind: &str) -> String {
    format!("cloud:synced_at:{}", kind)
}
// Per-kind conflict flag — set when a push returned 412 and the user
// hasn't resolved yet. Value stores the *remote* hash at conflict time,
// so the resolver can show summary stats against the right remote blob.
// Key format `cloud:conflict:<kind>`.
pub fn settings_key_conflict(kind: &str) -> String {
    format!("cloud:conflict:{}", kind)
}
