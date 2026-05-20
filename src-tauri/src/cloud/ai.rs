use sqlx::SqlitePool;

use crate::cloud::client::{get_json_auth, CloudError};
use crate::cloud::models::{CloudAiBalance, CloudAiUsage};

pub async fn get_balance(
    pool: &SqlitePool,
    token: &str,
    provider: &str,
) -> Result<CloudAiBalance, CloudError> {
    get_json_auth(pool, "/api/ai/balance", token, provider).await
}

pub async fn get_usage(
    pool: &SqlitePool,
    token: &str,
    provider: &str,
    limit: Option<u32>,
    before: Option<&str>,
) -> Result<CloudAiUsage, CloudError> {
    let limit = limit.unwrap_or(50).min(200);
    let mut path = format!("/api/ai/usage?limit={}", limit);
    if let Some(b) = before {
        let encoded = b.replace(':', "%3A");
        path.push_str("&before=");
        path.push_str(&encoded);
    }
    get_json_auth(pool, &path, token, provider).await
}
