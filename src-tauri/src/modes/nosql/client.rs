use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use mongodb::bson::{self, doc, Document};
use mongodb::options::{ClientOptions, Tls, TlsOptions};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoSqlConnectionConfig {
    pub name: String,
    pub driver: String,
    pub connection_string: String,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ssl: bool,
    #[serde(default)]
    pub direct_connection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoSqlQueryResult {
    pub documents: Vec<serde_json::Value>,
    pub total_count: Option<u64>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisKeyInfo {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisValue {
    pub key: String,
    pub key_type: String,
    pub value: serde_json::Value,
    pub ttl: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NoSqlConnection {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub connection_string: String,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub ssl: i32,
    pub direct_connection: i32,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

// ── Connection Pool ─────────────────────────────────────────────────────

pub enum NoSqlPool {
    Mongo(mongodb::Client),
    Redis(redis::aio::ConnectionManager),
}

pub type NoSqlConnections = Arc<Mutex<HashMap<String, NoSqlPool>>>;

pub fn create_nosql_state() -> NoSqlConnections {
    Arc::new(Mutex::new(HashMap::new()))
}

// ── Helper: build connection string if not provided ─────────────────────

fn build_mongo_uri(config: &NoSqlConnectionConfig) -> String {
    if !config.connection_string.is_empty() {
        return config.connection_string.clone();
    }
    let auth = match (&config.username, &config.password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => format!("{}:{}@", u, p),
        _ => String::new(),
    };
    let db_part = config
        .database
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(|d| format!("/{}", d))
        .unwrap_or_default();
    // When using auth, default authSource to admin (standard for root users)
    let auth_source = if !auth.is_empty() { "?authSource=admin" } else { "" };
    format!("mongodb://{}{}:{}{}{}", auth, config.host, config.port, db_part, auth_source)
}

fn build_redis_uri(config: &NoSqlConnectionConfig) -> String {
    if !config.connection_string.is_empty() {
        return config.connection_string.clone();
    }
    let scheme = if config.ssl { "rediss" } else { "redis" };
    let auth = match (&config.username, &config.password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => format!("{}:{}@", u, p),
        (None, Some(p)) | (Some(_), Some(p)) if !p.is_empty() => format!(":{}@", p),
        _ => String::new(),
    };
    let db = config
        .database
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(|d| format!("/{}", d))
        .unwrap_or_default();
    format!("{}://{}{}:{}{}", scheme, auth, config.host, config.port, db)
}

// ── Connection Management Commands ──────────────────────────────────────

#[tauri::command]
pub async fn nosql_connect(
    connections: State<'_, NoSqlConnections>,
    config: NoSqlConnectionConfig,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();

    let pool = match config.driver.as_str() {
        "mongodb" => {
            let uri = build_mongo_uri(&config);
            log::info!("MongoDB URI: {}", if uri.contains('@') {
                let at = uri.find('@').unwrap();
                format!("mongodb://***@{}", &uri[at+1..])
            } else { uri.clone() });
            let mut opts = ClientOptions::parse(&uri)
                .await
                .map_err(|e| format!("MongoDB connection parse error: {}", e))?;
            opts.server_selection_timeout = Some(std::time::Duration::from_secs(10));
            opts.connect_timeout = Some(std::time::Duration::from_secs(10));
            if config.direct_connection {
                opts.direct_connection = Some(true);
            }
            log::info!("MongoDB config: ssl={}, direct_connection={}", config.ssl, config.direct_connection);
            if config.ssl {
                let tls_opts = TlsOptions::builder()
                    .allow_invalid_certificates(Some(true))
                    .build();
                opts.tls = Some(Tls::Enabled(tls_opts));
                log::info!("MongoDB TLS enabled with allow_invalid_certificates=true");
            } else if opts.tls.is_none() {
                // No tls param in URI and SSL toggle is off — explicitly disable TLS
                opts.tls = Some(Tls::Disabled);
            }
            let client = mongodb::Client::with_options(opts)
                .map_err(|e| format!("MongoDB client error: {}", e))?;
            // Verify connectivity — use configured database or fallback to admin
            let ping_db = config.database.as_deref()
                .filter(|d| !d.is_empty())
                .unwrap_or("admin");
            client
                .database(ping_db)
                .run_command(doc! { "ping": 1 })
                .await
                .map_err(|e| format!("MongoDB ping failed: {}", e))?;
            NoSqlPool::Mongo(client)
        }
        "redis" => {
            let uri = build_redis_uri(&config);
            let client = redis::Client::open(uri.as_str())
                .map_err(|e| format!("Redis client error: {}", e))?;
            let cm = redis::aio::ConnectionManager::new(client)
                .await
                .map_err(|e| format!("Redis connection error: {}", e))?;
            NoSqlPool::Redis(cm)
        }
        other => return Err(format!("Unsupported driver: {}", other)),
    };

    connections.lock().await.insert(id.clone(), pool);
    Ok(id)
}

#[tauri::command]
pub async fn nosql_disconnect(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
) -> Result<(), String> {
    connections
        .lock()
        .await
        .remove(&connection_id)
        .ok_or_else(|| "Connection not found".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn nosql_test_connection(config: NoSqlConnectionConfig) -> Result<(), String> {
    match config.driver.as_str() {
        "mongodb" => {
            let uri = build_mongo_uri(&config);
            let mut opts = ClientOptions::parse(&uri)
                .await
                .map_err(|e| format!("MongoDB parse error: {}", e))?;
            opts.server_selection_timeout = Some(std::time::Duration::from_secs(10));
            opts.connect_timeout = Some(std::time::Duration::from_secs(10));
            if config.direct_connection {
                opts.direct_connection = Some(true);
            }
            if config.ssl {
                let tls_opts = TlsOptions::builder()
                    .allow_invalid_certificates(Some(true))
                    .build();
                opts.tls = Some(Tls::Enabled(tls_opts));
            } else if opts.tls.is_none() {
                opts.tls = Some(Tls::Disabled);
            }
            let client = mongodb::Client::with_options(opts)
                .map_err(|e| format!("MongoDB client error: {}", e))?;
            let ping_db = config.database.as_deref()
                .filter(|d| !d.is_empty())
                .unwrap_or("admin");
            client
                .database(ping_db)
                .run_command(doc! { "ping": 1 })
                .await
                .map_err(|e| format!("MongoDB ping failed: {}", e))?;
        }
        "redis" => {
            let uri = build_redis_uri(&config);
            let client = redis::Client::open(uri.as_str())
                .map_err(|e| format!("Redis client error: {}", e))?;
            let mut cm = redis::aio::ConnectionManager::new(client)
                .await
                .map_err(|e| format!("Redis connection error: {}", e))?;
            redis::cmd("PING")
                .query_async::<String>(&mut cm)
                .await
                .map_err(|e| format!("Redis ping failed: {}", e))?;
        }
        other => return Err(format!("Unsupported driver: {}", other)),
    }
    Ok(())
}

// ── MongoDB Commands ────────────────────────────────────────────────────

macro_rules! get_mongo {
    ($conns:expr, $id:expr) => {{
        let guard = $conns.lock().await;
        match guard.get($id) {
            Some(NoSqlPool::Mongo(c)) => c.clone(),
            Some(_) => return Err("Connection is not MongoDB".to_string()),
            None => return Err("Connection not found".to_string()),
        }
    }};
}

macro_rules! get_redis {
    ($conns:expr, $id:expr) => {{
        let guard = $conns.lock().await;
        match guard.get($id) {
            Some(NoSqlPool::Redis(cm)) => cm.clone(),
            Some(_) => return Err("Connection is not Redis".to_string()),
            None => return Err("Connection not found".to_string()),
        }
    }};
}

#[tauri::command]
pub async fn nosql_list_databases(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
) -> Result<Vec<String>, String> {
    let client = get_mongo!(connections, &connection_id);
    let all = client
        .list_database_names()
        .await
        .map_err(|e| format!("Failed to list databases: {}", e))?;
    let mut filtered: Vec<String> = all
        .into_iter()
        .filter(|d| d != "admin" && d != "config" && d != "local")
        .collect();
    filtered.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    Ok(filtered)
}

#[tauri::command]
pub async fn nosql_list_collections(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
) -> Result<Vec<String>, String> {
    let client = get_mongo!(connections, &connection_id);
    let mut names = client
        .database(&database)
        .list_collection_names()
        .await
        .map_err(|e| format!("Failed to list collections: {}", e))?;
    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    Ok(names)
}

fn parse_bson_doc(json_str: &str) -> Result<Document, String> {
    let val: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;
    let val = auto_convert_objectids(val);
    let bson_val = bson::to_bson(&val).map_err(|e| format!("BSON conversion error: {}", e))?;
    match bson_val {
        bson::Bson::Document(d) => Ok(d),
        _ => Err("Expected a JSON object".to_string()),
    }
}

/// Auto-convert plain hex strings for _id fields to MongoDB Extended JSON ObjectId format.
/// e.g. {"_id": "611fa947ea92124f4d5dfbcd"} → {"_id": {"$oid": "611fa947ea92124f4d5dfbcd"}}
fn auto_convert_objectids(val: serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Object(mut map) => {
            if let Some(id_val) = map.get("_id") {
                if let Some(id_str) = id_val.as_str() {
                    // 24-char hex string → ObjectId
                    if id_str.len() == 24 && id_str.chars().all(|c| c.is_ascii_hexdigit()) {
                        let mut oid = serde_json::Map::new();
                        oid.insert("$oid".to_string(), serde_json::Value::String(id_str.to_string()));
                        map.insert("_id".to_string(), serde_json::Value::Object(oid));
                    }
                }
            }
            serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, auto_convert_objectids(v)))
                    .collect(),
            )
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(auto_convert_objectids).collect())
        }
        other => other,
    }
}

fn doc_to_json(doc: &Document) -> serde_json::Value {
    // Use extended JSON then parse back to get clean serde_json::Value
    let s = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
pub async fn nosql_find_documents(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    filter: String,
    sort: Option<String>,
    limit: Option<i64>,
    skip: Option<u64>,
) -> Result<NoSqlQueryResult, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);

    let filter_doc = parse_bson_doc(&filter)?;
    let start = Instant::now();

    let total_count = coll
        .count_documents(filter_doc.clone())
        .await
        .ok();

    let mut find = coll.find(filter_doc);
    if let Some(sort_str) = sort {
        find = find.sort(parse_bson_doc(&sort_str)?);
    }
    if let Some(l) = limit {
        find = find.limit(l);
    }
    if let Some(s) = skip {
        find = find.skip(s);
    }

    let mut cursor = find.await.map_err(|e| format!("Find error: {}", e))?;

    let mut documents = Vec::new();
    while cursor
        .advance()
        .await
        .map_err(|e| format!("Cursor error: {}", e))?
    {
        let doc = cursor
            .deserialize_current()
            .map_err(|e| format!("Deserialize error: {}", e))?;
        documents.push(doc_to_json(&doc));
    }

    Ok(NoSqlQueryResult {
        documents,
        total_count,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn nosql_insert_document(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    document: String,
) -> Result<String, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);
    let doc = parse_bson_doc(&document)?;
    let result = coll
        .insert_one(doc)
        .await
        .map_err(|e| format!("Insert error: {}", e))?;
    Ok(result.inserted_id.to_string())
}

#[tauri::command]
pub async fn nosql_update_document(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    filter: String,
    update: String,
) -> Result<u64, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);
    let filter_doc = parse_bson_doc(&filter)?;
    let update_doc = parse_bson_doc(&update)?;
    let result = coll
        .update_many(filter_doc, update_doc)
        .await
        .map_err(|e| format!("Update error: {}", e))?;
    Ok(result.modified_count)
}

#[tauri::command]
pub async fn nosql_delete_document(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    filter: String,
) -> Result<u64, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);
    let filter_doc = parse_bson_doc(&filter)?;
    let result = coll
        .delete_many(filter_doc)
        .await
        .map_err(|e| format!("Delete error: {}", e))?;
    Ok(result.deleted_count)
}

#[tauri::command]
pub async fn nosql_count_documents(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    filter: Option<String>,
) -> Result<u64, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);
    let filter_doc = match filter {
        Some(f) => parse_bson_doc(&f)?,
        None => doc! {},
    };
    coll.count_documents(filter_doc)
        .await
        .map_err(|e| format!("Count error: {}", e))
}

#[tauri::command]
pub async fn nosql_aggregate(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    pipeline: String,
) -> Result<NoSqlQueryResult, String> {
    let client = get_mongo!(connections, &connection_id);
    let coll = client.database(&database).collection::<Document>(&collection);

    let pipeline_val: Vec<serde_json::Value> =
        serde_json::from_str(&pipeline).map_err(|e| format!("Invalid pipeline JSON: {}", e))?;

    let bson_pipeline: Vec<Document> = pipeline_val
        .into_iter()
        .map(|v| {
            let v = auto_convert_objectids(v);
            let bv = bson::to_bson(&v).map_err(|e| format!("BSON error: {}", e))?;
            match bv {
                bson::Bson::Document(d) => Ok(d),
                _ => Err("Pipeline stage must be an object".to_string()),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let start = Instant::now();
    let mut cursor = coll
        .aggregate(bson_pipeline)
        .await
        .map_err(|e| format!("Aggregate error: {}", e))?;

    let mut documents = Vec::new();
    while cursor
        .advance()
        .await
        .map_err(|e| format!("Cursor error: {}", e))?
    {
        let doc = cursor
            .deserialize_current()
            .map_err(|e| format!("Deserialize error: {}", e))?;
        documents.push(doc_to_json(&doc));
    }

    Ok(NoSqlQueryResult {
        documents,
        total_count: None,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

// ── Database & Collection Management Commands ─────────────────────────

#[tauri::command]
pub async fn nosql_create_collection(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
) -> Result<(), String> {
    let client = get_mongo!(connections, &connection_id);
    client
        .database(&database)
        .create_collection(&collection)
        .await
        .map_err(|e| format!("Create collection error: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn nosql_drop_database(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
) -> Result<(), String> {
    let client = get_mongo!(connections, &connection_id);
    client
        .database(&database)
        .drop()
        .await
        .map_err(|e| format!("Drop database error: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn nosql_drop_collection(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
) -> Result<(), String> {
    let client = get_mongo!(connections, &connection_id);
    client
        .database(&database)
        .collection::<Document>(&collection)
        .drop()
        .await
        .map_err(|e| format!("Drop collection error: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn nosql_rename_collection(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    database: String,
    collection: String,
    new_name: String,
) -> Result<(), String> {
    let client = get_mongo!(connections, &connection_id);
    let admin_db = client.database("admin");
    admin_db.run_command(doc! {
        "renameCollection": format!("{}.{}", database, collection),
        "to": format!("{}.{}", database, new_name),
    })
    .await
    .map_err(|e| format!("Rename collection error: {}", e))?;
    Ok(())
}

// ── Redis Commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn redis_execute(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    command: String,
) -> Result<String, String> {
    let mut cm = get_redis!(connections, &connection_id);

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut cmd = redis::cmd(parts[0]);
    for arg in &parts[1..] {
        cmd.arg(*arg);
    }

    let result: redis::Value = cmd
        .query_async(&mut cm)
        .await
        .map_err(|e| format!("Redis error: {}", e))?;

    Ok(format_redis_value(&result))
}

fn format_redis_value(val: &redis::Value) -> String {
    match val {
        redis::Value::Nil => "(nil)".to_string(),
        redis::Value::Int(i) => format!("(integer) {}", i),
        redis::Value::BulkString(data) => String::from_utf8_lossy(data).to_string(),
        redis::Value::Array(arr) => {
            let items: Vec<String> = arr
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{}) {}", i + 1, format_redis_value(v)))
                .collect();
            items.join("\n")
        }
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Okay => "OK".to_string(),
        redis::Value::Double(f) => format!("(double) {}", f),
        redis::Value::Boolean(b) => format!("(boolean) {}", b),
        redis::Value::VerbatimString { format: _, text } => text.clone(),
        redis::Value::BigNumber(n) => format!("(bignumber) {}", n),
        redis::Value::Map(entries) => {
            let items: Vec<String> = entries
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    format!(
                        "{}) {} -> {}",
                        i + 1,
                        format_redis_value(k),
                        format_redis_value(v)
                    )
                })
                .collect();
            items.join("\n")
        }
        redis::Value::Set(items) => {
            let strs: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{}) {}", i + 1, format_redis_value(v)))
                .collect();
            strs.join("\n")
        }
        redis::Value::Attribute { data, attributes: _ } => format_redis_value(data),
        redis::Value::Push { kind: _, data } => {
            let items: Vec<String> = data
                .iter()
                .map(|v| format_redis_value(v))
                .collect();
            items.join("\n")
        }
        redis::Value::ServerError(e) => format!("(error) {:?}", e),
    }
}

#[tauri::command]
pub async fn redis_list_keys(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    pattern: Option<String>,
) -> Result<Vec<RedisKeyInfo>, String> {
    let mut cm = get_redis!(connections, &connection_id);
    let pat = pattern.unwrap_or_else(|| "*".to_string());

    let keys: Vec<String> = redis::cmd("SCAN")
        .arg(0)
        .arg("MATCH")
        .arg(&pat)
        .arg("COUNT")
        .arg(1000)
        .query_async(&mut cm)
        .await
        .map_err(|e| format!("SCAN error: {}", e))?;

    let mut result = Vec::new();
    for key in keys {
        let key_type: String = redis::cmd("TYPE")
            .arg(&key)
            .query_async(&mut cm)
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        let ttl: i64 = cm.ttl(&key).await.unwrap_or(-2);
        result.push(RedisKeyInfo {
            key,
            key_type,
            ttl,
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn redis_get_key(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    key: String,
) -> Result<RedisValue, String> {
    let mut cm = get_redis!(connections, &connection_id);

    let key_type: String = redis::cmd("TYPE")
        .arg(&key)
        .query_async(&mut cm)
        .await
        .map_err(|e| format!("TYPE error: {}", e))?;

    let ttl: i64 = cm.ttl(&key).await.unwrap_or(-2);

    let value: serde_json::Value = match key_type.as_str() {
        "string" => {
            let v: String = cm.get(&key).await.map_err(|e| format!("GET error: {}", e))?;
            serde_json::Value::String(v)
        }
        "list" => {
            let v: Vec<String> = cm
                .lrange(&key, 0, -1)
                .await
                .map_err(|e| format!("LRANGE error: {}", e))?;
            serde_json::json!(v)
        }
        "set" => {
            let v: Vec<String> = cm
                .smembers(&key)
                .await
                .map_err(|e| format!("SMEMBERS error: {}", e))?;
            serde_json::json!(v)
        }
        "hash" => {
            let v: HashMap<String, String> = cm
                .hgetall(&key)
                .await
                .map_err(|e| format!("HGETALL error: {}", e))?;
            serde_json::json!(v)
        }
        "zset" => {
            let v: Vec<(String, f64)> = cm
                .zrange_withscores(&key, 0, -1)
                .await
                .map_err(|e| format!("ZRANGE error: {}", e))?;
            let entries: Vec<serde_json::Value> = v
                .into_iter()
                .map(|(member, score)| serde_json::json!({"member": member, "score": score}))
                .collect();
            serde_json::json!(entries)
        }
        _ => serde_json::Value::Null,
    };

    Ok(RedisValue {
        key,
        key_type,
        value,
        ttl,
    })
}

#[tauri::command]
pub async fn redis_set_key(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    key: String,
    value: String,
    ttl: Option<i64>,
) -> Result<(), String> {
    let mut cm = get_redis!(connections, &connection_id);

    if let Some(seconds) = ttl {
        cm.set_ex::<_, _, ()>(&key, &value, seconds as u64)
            .await
            .map_err(|e| format!("SETEX error: {}", e))?;
    } else {
        cm.set::<_, _, ()>(&key, &value)
            .await
            .map_err(|e| format!("SET error: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn redis_delete_key(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
    key: String,
) -> Result<(), String> {
    let mut cm = get_redis!(connections, &connection_id);
    cm.del::<_, ()>(&key)
        .await
        .map_err(|e| format!("DEL error: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn redis_get_info(
    connections: State<'_, NoSqlConnections>,
    connection_id: String,
) -> Result<String, String> {
    let mut cm = get_redis!(connections, &connection_id);
    redis::cmd("INFO")
        .query_async::<String>(&mut cm)
        .await
        .map_err(|e| format!("INFO error: {}", e))
}

// ── Saved Connections (SQLite persistence) ──────────────────────────────

#[tauri::command]
pub async fn nosql_save_connection(
    pool: State<'_, SqlitePool>,
    config: NoSqlConnectionConfig,
) -> Result<NoSqlConnection, String> {
    use crate::shared::repos::nosql_connections as nosql_repo;
    let id = Uuid::new_v4().to_string();

    let max_order = nosql_repo::max_sort_order(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    nosql_repo::insert(
        pool.inner(),
        &id,
        &config.name,
        &config.driver,
        &config.connection_string,
        &config.host,
        config.port as i32,
        config.database.as_deref().unwrap_or(""),
        config.username.as_deref().unwrap_or(""),
        config.password.as_deref().unwrap_or(""),
        if config.ssl { 1 } else { 0 },
        if config.direct_connection { 1 } else { 0 },
        max_order.0 + 1,
    )
    .await
    .map_err(|e| e.to_string())?;

    nosql_repo::get_by_id(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nosql_list_saved_connections(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<NoSqlConnection>, String> {
    use crate::shared::repos::nosql_connections as nosql_repo;
    nosql_repo::list_all(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nosql_delete_saved_connection(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    use crate::shared::repos::nosql_connections as nosql_repo;
    nosql_repo::delete_by_id(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn nosql_update_saved_connection(
    pool: State<'_, SqlitePool>,
    id: String,
    config: NoSqlConnectionConfig,
) -> Result<NoSqlConnection, String> {
    use crate::shared::repos::nosql_connections as nosql_repo;
    nosql_repo::update(
        pool.inner(),
        &id,
        &config.name,
        &config.driver,
        &config.connection_string,
        &config.host,
        config.port as i32,
        config.database.as_deref().unwrap_or(""),
        config.username.as_deref().unwrap_or(""),
        config.password.as_deref().unwrap_or(""),
        if config.ssl { 1 } else { 0 },
        if config.direct_connection { 1 } else { 0 },
    )
    .await
    .map_err(|e| e.to_string())?;

    nosql_repo::get_by_id(pool.inner(), &id)
        .await
        .map_err(|e| e.to_string())
}
