import { invoke } from '@tauri-apps/api/core';
import type { SqlConnectionConfig, SqlConnection, SqlQueryResult, TableInfo, ColumnInfo, SqlScript } from './types';

// Pool lifecycle — every pool is keyed by `(connId, database)` on the
// backend; the frontend never sees a live UUID anymore.

export async function sqlEnsurePool(connId: string, database: string): Promise<string> {
  return invoke('sql_ensure_pool', { connId, database });
}

export async function sqlDisconnectPool(connId: string, database: string): Promise<void> {
  return invoke('sql_disconnect_pool', { connId, database });
}

export async function sqlDisconnectConnection(connId: string): Promise<void> {
  return invoke('sql_disconnect_connection', { connId });
}

export async function sqlTestConnection(config: SqlConnectionConfig): Promise<string> {
  return invoke('sql_test_connection', { config });
}

// Query execution — caller supplies a `queryId` (UUID) so cancel can target it.

export async function sqlExecuteQuery(
  connId: string,
  database: string,
  query: string,
  queryId: string,
): Promise<SqlQueryResult> {
  return invoke('sql_execute_query', { connId, database, query, queryId });
}

export async function sqlCancelQuery(queryId: string): Promise<void> {
  return invoke('sql_cancel_query', { queryId });
}

// Introspection — all take `(connId, database)`.

export async function sqlListDatabases(connId: string, database: string): Promise<string[]> {
  return invoke('sql_list_databases', { connId, database });
}

/** Run all statements inside a single transaction (PG/MySQL/SQLite). On
 *  ClickHouse and D1 they run sequentially with no rollback — each
 *  driver's batch function surfaces that in the error message if anything
 *  fails. Returns one SqlQueryResult per statement on success. */
export async function sqlExecuteBatch(
  connId: string,
  database: string,
  statements: string[],
): Promise<SqlQueryResult[]> {
  return invoke('sql_execute_batch', { connId, database, statements });
}

export async function sqlCreateDatabase(connId: string, database: string, name: string): Promise<void> {
  return invoke('sql_create_database', { connId, database, name });
}

export async function sqlListSchemas(connId: string, database: string): Promise<string[]> {
  return invoke('sql_list_schemas', { connId, database });
}

/** Return the resolved default schema (first writable entry in Postgres's
 *  search_path, or null for engines without schemas). Used by the editor
 *  to make unqualified completion work for connections whose tables
 *  don't live in `public`. */
export async function sqlCurrentSchema(connId: string, database: string): Promise<string | null> {
  return invoke('sql_current_schema', { connId, database });
}

export async function sqlListTables(
  connId: string,
  database: string,
  schema?: string,
): Promise<TableInfo[]> {
  return invoke('sql_list_tables', { connId, database, schema });
}

export async function sqlDescribeTable(
  connId: string,
  database: string,
  table: string,
  schema?: string,
): Promise<ColumnInfo[]> {
  return invoke('sql_describe_table', { connId, database, table, schema });
}

// Saved connection profile CRUD (unchanged).

export async function sqlSaveConnection(config: SqlConnectionConfig): Promise<SqlConnection> {
  return invoke('sql_save_connection', { config });
}

export async function sqlListSavedConnections(): Promise<SqlConnection[]> {
  return invoke('sql_list_saved_connections');
}

export async function sqlDeleteSavedConnection(id: string): Promise<void> {
  return invoke('sql_delete_saved_connection', { id });
}

export async function sqlUpdateSavedConnection(id: string, config: SqlConnectionConfig): Promise<SqlConnection> {
  return invoke('sql_update_saved_connection', { id, config });
}

// Saved script CRUD (unchanged).

export async function sqlSaveScript(name: string, connectionId: string | null, databaseName: string, query: string): Promise<SqlScript> {
  return invoke('sql_save_script', { name, connectionId, databaseName, query });
}

export async function sqlListScripts(): Promise<SqlScript[]> {
  return invoke('sql_list_scripts');
}

export async function sqlUpdateScript(
  id: string,
  name: string,
  query: string,
  databaseName?: string,
  connectionId?: string,
): Promise<SqlScript> {
  return invoke('sql_update_script', { id, name, query, databaseName, connectionId });
}

export async function sqlDeleteScript(id: string): Promise<void> {
  return invoke('sql_delete_script', { id });
}
