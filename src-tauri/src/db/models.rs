use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sort_order: i32,
    pub env_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub description: String,
    pub method: String,
    pub url: String,
    pub body: String,
    pub body_type: String,
    pub auth_type: String,
    pub auth_data: String,
    pub pre_script: String,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RequestHeader {
    pub id: String,
    pub request_id: String,
    pub key: String,
    pub value: String,
    pub enabled: i32,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RequestParam {
    pub id: String,
    pub request_id: String,
    pub key: String,
    pub value: String,
    pub enabled: i32,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_default: i32,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct EnvVariable {
    pub id: String,
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub is_secret: i32,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub request_id: Option<String>,
    /// Joined from `requests.name`. NULL for unsaved/quick-execute entries
    /// or when the source request has been deleted.
    pub request_name: Option<String>,
    pub method: String,
    pub url: String,
    pub resolved_url: String,
    pub request_body: String,
    pub request_headers: String,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub response_headers: Option<String>,
    pub response_size_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub environment_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub key: String,
    pub value: String,
}

/// Helper to load environment variables as a HashMap for variable resolution.
pub async fn load_env_vars(
    pool: &sqlx::SqlitePool,
    environment_id: &str,
) -> std::collections::HashMap<String, String> {
    let vars = sqlx::query_as::<_, EnvVariable>(
        "SELECT * FROM env_variables WHERE environment_id = ?",
    )
    .bind(environment_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut map = std::collections::HashMap::new();
    for var in vars {
        map.insert(var.key, var.value);
    }
    map
}
