mod appearance;
mod commands;
mod db;
mod github;

use std::str::FromStr;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:qorix.db", db::migrations::get_migrations())
                .build(),
        )
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Set rounded corners + dock icon
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSApp, NSApplication, NSImage, NSWindow};
                use cocoa::base::{nil, id};
                use cocoa::foundation::NSData;

                // Round the window corners
                if let Some(win) = app.get_webview_window("main") {
                    use objc::{runtime::Object, sel, sel_impl};
                    let ns_win: *mut Object = win.ns_window().unwrap() as *mut Object;
                    unsafe {
                        let _: () = objc::msg_send![ns_win, setHasShadow: true];
                        let content_view: *mut Object = objc::msg_send![ns_win, contentView];
                        let _: () = objc::msg_send![content_view, setWantsLayer: true];
                        let layer: *mut Object = objc::msg_send![content_view, layer];
                        let _: () = objc::msg_send![layer, setCornerRadius: 10.0_f64];
                        let _: () = objc::msg_send![layer, setMasksToBounds: true];
                    }
                }
                let icon_data = include_bytes!("../icons/icon.png");
                unsafe {
                    let ns_data = NSData::dataWithBytes_length_(
                        nil,
                        icon_data.as_ptr() as *const std::ffi::c_void,
                        icon_data.len() as u64,
                    );
                    let ns_image = NSImage::initWithData_(NSImage::alloc(nil), ns_data);
                    NSApp().setApplicationIconImage_(ns_image);
                }
            }

            // Initialize sqlx connection pool for Rust commands
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();
            let db_path = app_data_dir.join("qorix.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

            let pool = tauri::async_runtime::block_on(async {
                let opts = sqlx::sqlite::SqliteConnectOptions::from_str(&db_url)
                    .expect("invalid db url")
                    .pragma("foreign_keys", "ON")
                    .create_if_missing(true);
                sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect_with(opts)
                    .await
                    .expect("failed to connect to database")
            });

            // Run schema migrations directly via sqlx
            tauri::async_runtime::block_on(async {
                for migration in db::migrations::get_migrations() {
                    for statement in migration.sql.split(';') {
                        let stmt = statement.trim();
                        if !stmt.is_empty() {
                            if let Err(e) = sqlx::query(stmt).execute(&pool).await {
                                // Ignore "already exists" / "duplicate column" errors from re-running migrations
                                let err_str = e.to_string();
                                if !err_str.contains("already exists") && !err_str.contains("duplicate column") {
                                    eprintln!("Migration v{} statement failed: {}", migration.version, e);
                                }
                            }
                        }
                    }
                }
            });

            // Load saved vibrancy material before managing pool (which moves it)
            let saved_material = tauri::async_runtime::block_on(async {
                sqlx::query_as::<_, (String,)>(
                    "SELECT value FROM settings WHERE key = 'vibrancy_material'",
                )
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
                .map(|r| r.0)
                .unwrap_or_else(|| "sidebar".to_string())
            });

            app.manage(pool);
            app.manage(Arc::new(commands::sql_client::SqlConnectionManager::new()));
            app.manage(commands::nosql_client::create_nosql_state());
            app.manage(commands::agent_models::TerminalState::default());

            // Apply vibrancy on macOS — use Sidebar material (what native macOS apps use)
            if let Some(window) = app.get_webview_window("main") {
                let _ = appearance::vibrancy::apply_vibrancy(&window, &saved_material);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::collections::list_collections,
            commands::collections::create_collection,
            commands::collections::update_collection,
            commands::collections::delete_collection,
            commands::collections::reorder_collections,
            commands::requests::list_requests,
            commands::requests::get_request,
            commands::requests::create_request,
            commands::requests::update_request,
            commands::requests::delete_request,
            commands::requests::duplicate_request,
            commands::requests::move_request,
            commands::requests::update_request_headers,
            commands::requests::update_request_params,
            commands::environments::list_environments,
            commands::environments::create_environment,
            commands::environments::update_environment,
            commands::environments::delete_environment,
            commands::environments::set_default_environment,
            commands::environments::list_env_variables,
            commands::environments::set_env_variable,
            commands::environments::update_env_variable,
            commands::environments::delete_env_variable,
            commands::environments::get_env_variables_for_resolution,
            commands::http_executor::execute_request,
            commands::http_executor::quick_execute,
            commands::history::list_history,
            commands::history::clear_history,
            commands::history::delete_history_entry,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            appearance::vibrancy::set_vibrancy,
            appearance::vibrancy::get_appearance,
            appearance::vibrancy::set_appearance,
            appearance::vibrancy::get_available_themes,
            github::oauth::github_connect,
            github::oauth::github_disconnect,
            github::oauth::github_get_status,
            github::oauth::github_get_oauth_url,
            github::oauth::github_connect_with_token,
            github::gist::gist_check_exists,
            github::gist::gist_sync_push,
            github::gist::gist_sync_pull,
            commands::import_export::export_collection,
            commands::import_export::export_all_collections,
            commands::import_export::import_qorix,
            commands::import_export::import_postman,
            commands::import_export::import_curl,
            commands::import_export::export_as_curl,
            commands::sql_client::sql_connect,
            commands::sql_client::sql_connect_database,
            commands::sql_client::sql_disconnect,
            commands::sql_client::sql_test_connection,
            commands::sql_client::sql_execute_query,
            commands::sql_client::sql_list_databases,
            commands::sql_client::sql_create_database,
            commands::sql_client::sql_list_schemas,
            commands::sql_client::sql_list_tables,
            commands::sql_client::sql_describe_table,
            commands::sql_client::sql_save_connection,
            commands::sql_client::sql_list_saved_connections,
            commands::sql_client::sql_delete_saved_connection,
            commands::sql_client::sql_update_saved_connection,
            commands::sql_client::sql_save_script,
            commands::sql_client::sql_list_scripts,
            commands::sql_client::sql_update_script,
            commands::sql_client::sql_delete_script,
            commands::nosql_client::nosql_connect,
            commands::nosql_client::nosql_disconnect,
            commands::nosql_client::nosql_test_connection,
            commands::nosql_client::nosql_list_databases,
            commands::nosql_client::nosql_list_collections,
            commands::nosql_client::nosql_find_documents,
            commands::nosql_client::nosql_insert_document,
            commands::nosql_client::nosql_update_document,
            commands::nosql_client::nosql_delete_document,
            commands::nosql_client::nosql_count_documents,
            commands::nosql_client::nosql_aggregate,
            commands::nosql_client::nosql_create_collection,
            commands::nosql_client::nosql_drop_database,
            commands::nosql_client::nosql_drop_collection,
            commands::nosql_client::nosql_rename_collection,
            commands::nosql_client::redis_execute,
            commands::nosql_client::redis_list_keys,
            commands::nosql_client::redis_get_key,
            commands::nosql_client::redis_set_key,
            commands::nosql_client::redis_delete_key,
            commands::nosql_client::redis_get_info,
            commands::nosql_client::nosql_save_connection,
            commands::nosql_client::nosql_list_saved_connections,
            commands::nosql_client::nosql_delete_saved_connection,
            commands::nosql_client::nosql_update_saved_connection,
            commands::ai::test_ai_key,
            commands::ai::get_ai_usage_stats,
            commands::ai::get_ai_provider_stats,
            commands::ai::reset_ai_usage,
            commands::ai::record_ai_usage,
            commands::ai::ai_chat,
            // Agent mode
            commands::agent::agent_list_sessions,
            commands::agent::agent_create_session,
            commands::agent::agent_update_session,
            commands::agent::agent_delete_session,
            commands::agent::agent_update_session_id,
            commands::agent::agent_update_last_used,
            commands::agent::agent_update_worktree,
            commands::agent::agent_list_contexts,
            commands::agent::agent_save_context,
            commands::agent::agent_delete_context,
            commands::agent::agent_get_session_contexts,
            commands::agent::agent_attach_context,
            commands::agent::agent_detach_context,
            commands::agent_terminal::agent_spawn_terminal,
            commands::agent_terminal::agent_spawn_shell,
            commands::agent_terminal::agent_write_to_terminal,
            commands::agent_terminal::agent_resize_terminal,
            commands::agent_terminal::agent_kill_terminal,
            commands::agent_worktree::agent_is_git_repo,
            commands::agent_worktree::agent_create_worktree,
            commands::agent_worktree::agent_remove_worktree,
            commands::agent_git::agent_git_status,
            commands::agent_git::agent_git_branch,
            commands::agent_git::agent_git_ahead_behind,
            commands::agent_git::agent_git_commit,
            commands::agent_git::agent_git_push,
            commands::agent_git::agent_git_pull,
            commands::agent_git::agent_git_diff_file,
            commands::agent_git::agent_git_stage_file,
            commands::agent_git::agent_git_unstage_file,
            commands::agent_git::agent_git_log,
            commands::agent_git::agent_git_stash,
            commands::agent_git::agent_git_stash_pop,
            commands::agent_git::agent_git_list_branches,
            commands::agent_git::agent_git_switch_branch,
            commands::agent_plugins::agent_get_plugins,
            commands::agent_plugins::agent_toggle_plugin,
            commands::agent_plugins::agent_get_marketplace_plugins,
            commands::agent_plugins::agent_install_plugin,
            commands::agent_plugins::agent_uninstall_plugin,
            commands::agent_usage::agent_get_usage_analytics,
            commands::agent_usage::agent_fetch_usage_limits,
            commands::agent_usage::agent_discover_sessions,
            commands::agent_usage::agent_get_session_tokens,
            commands::agent_usage::agent_get_session_context_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
