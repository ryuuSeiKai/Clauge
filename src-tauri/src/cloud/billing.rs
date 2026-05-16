use sqlx::SqlitePool;

use crate::cloud::client::{get_json_no_auth, post_json_auth, CloudError};
use crate::cloud::models::{CloudCheckoutRequest, CloudCheckoutResponse, CloudPortalResponse, CloudPricing};

pub async fn get_pricing(pool: &SqlitePool) -> Result<CloudPricing, CloudError> {
    get_json_no_auth(pool, "/api/billing/pricing").await
}

pub async fn create_checkout(
    pool: &SqlitePool,
    token: &str,
    provider: &str,
    plan: &str,
) -> Result<CloudCheckoutResponse, CloudError> {
    let body = serde_json::to_value(CloudCheckoutRequest { plan: plan.to_string() })
        .map_err(|e| CloudError::Network(e.to_string()))?;
    post_json_auth(pool, "/api/billing/checkout", body, token, provider).await
}

pub async fn create_portal_session(
    pool: &SqlitePool,
    token: &str,
    provider: &str,
) -> Result<CloudPortalResponse, CloudError> {
    post_json_auth(
        pool,
        "/api/billing/portal",
        serde_json::Value::Object(serde_json::Map::new()),
        token,
        provider,
    )
    .await
}
