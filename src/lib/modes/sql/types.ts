import type { SqlDialectKey } from './dialects';

// Re-export `SqlDialectKey` under its legacy name `SqlDriver` so existing
// import sites keep compiling. The canonical definition lives in `./dialects`.
export type { SqlDialectKey as SqlDriver } from './dialects';

export interface SqlConnectionConfig {
  name: string;
  driver: SqlDialectKey;
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  ssl: boolean;
  /** Optional SSH profile ID — when set, the runtime opens a tunnel through
   * that profile and routes the DB connection through it. NULL = direct. */
  sshProfileId?: string | null;
}

export interface SqlConnection {
  id: string;
  name: string;
  driver: SqlDialectKey;
  host: string;
  port: number;
  databaseName: string;
  username: string;
  password: string;
  ssl: number;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  sshProfileId?: string | null;
}

/** Verb classification surfaced by the backend so the result UI knows
 *  whether to render the row grid (Select / Unknown) or the side-effect
 *  stats card (Dml / Ddl). Was previously inferred from `columns.length`,
 *  which misclassified empty SELECTs whose describe() failed. */
export type QueryKind = 'select' | 'dml' | 'ddl' | 'unknown';

export interface SqlQueryResult {
  columns: string[];
  rows: unknown[][];
  affectedRows: number;
  durationMs: number;
  queryKind?: QueryKind;
}

export interface SqlResultEntry {
  label: string;
  query: string;
  result: SqlQueryResult | null;
  error: string | null;
  connectionId?: string;  // live pool ID — used when AI auto-connects to a different database
  /** Epoch ms when this statement started executing. Drives the DML
   *  Statistics panel (Start / Finish = startedAt + result.durationMs). */
  startedAt?: number;
}

/** Per-tab binding: which `(connection, database)` this tab queries. */
export interface Binding {
  connectionId: string;
  database: string;
}

/** Per-tab in-flight query metadata — drives Cancel + elapsed timer UI. */
export interface InFlight {
  queryId: string;
  startedAt: number;
}

/** State of the `(connectionId, database)` pool keyed by `${connId}:${db}`. */
export type PoolState = 'idle' | 'connecting' | 'connected' | 'error';

export interface TableInfo {
  name: string;
  tableType: string;
  rowCount: number;
  /** Schema namespace this table lives in. Postgres only — undefined for
   *  MySQL/SQLite/ClickHouse/D1. When set, `buildSchema()` emits both
   *  `tableName` and `schema.tableName` keys so the editor can complete
   *  qualified references like `analytics.events.id`. */
  schema?: string;
}

export interface ColumnInfo {
  name: string;
  dataType: string;
  isNullable: boolean;
  isPrimaryKey: boolean;
  defaultValue: string | null;
}

export interface SqlScript {
  id: string;
  name: string;
  connectionId: string | null;
  databaseName: string;
  query: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}
