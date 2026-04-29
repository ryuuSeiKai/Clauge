// NoSQL mode — owns the MongoDB and Redis connection pools, saved-connection
// CRUD, and the AI tool implementations that drive document and key/value
// queries from the chat surface.
//
// `client` hosts the `#[tauri::command]` handlers and the shared
// `NoSqlConnections` state; lib.rs references them as
// `crate::modes::nosql::client::*`.
// `ai_tools` mixes app-SQLite reads with MongoDB/Redis driver calls and is
// invoked from `commands::ai::tools` via `crate::modes::nosql::ai_tools::*`.

pub mod client;
pub mod ai_tools;
