import { writable, derived, get } from 'svelte/store';
import type { SqlConnection, SqlConnectionConfig, TableInfo, SqlQueryResult, SqlResultEntry, SqlScript } from './types';
import * as sqlCmd from './commands';
import { getSqlRowLimit, setSqlRowLimit } from '$lib/shared/constants/storage';

export const connections = writable<SqlConnection[]>([]);
export const activeConnectionId = writable<string | null>(null);
export const connectedIds = writable<Set<string>>(new Set());
// Maps saved connection ID -> live connection ID (from Rust pool)
export const liveConnectionIds = writable<Record<string, string>>({});
// Maps "savedConnId:dbName" -> live connection ID for per-database connections
export const dbLiveConnections = writable<Record<string, string>>({});
export const sqlScripts = writable<SqlScript[]>([]);
export const showSqlConnectionDialog = writable(false);
export const editingSqlConnection = writable<SqlConnection | null>(null);
export const showSqlDisconnectConfirm = writable(false);
export const sqlDisconnectTarget = writable<SqlConnection | null>(null);

// New stores for tree nav
export const expandedConnectionId = writable<string | null>(null);
export const selectedDatabase = writable<string>('');
export const connectionDatabases = writable<Map<string, string[]>>(new Map());
export const databaseTables = writable<Map<string, TableInfo[]>>(new Map());

// Derived: is the active connection connected?
export const sqlIsConnected = derived(
  [activeConnectionId, connectedIds],
  ([$activeConnectionId, $connectedIds]) =>
    $activeConnectionId ? $connectedIds.has($activeConnectionId) : false
);

// Default row limit for SELECT queries (0 = no limit).
// `getSqlRowLimit` migrates the legacy `sqlRowLimit` key on first read.
const SQL_ROW_LIMIT_DEFAULT = 100;
export const sqlRowLimit = writable<number>(getSqlRowLimit(SQL_ROW_LIMIT_DEFAULT));
sqlRowLimit.subscribe(v => setSqlRowLimit(v));

// Event for inserting query from nav tree or AI
export const insertQueryText = writable<string>('');

// AI helper — insert query into active SQL editor
export function applyAiQuery(query: string) {
  insertQueryText.set(query);
}

// --- SQL Tab State (per global tab) ---
export interface SqlTabData {
  query: string;
  result: SqlQueryResult | null;
  error: string | null;
  loading: boolean;
  database: string;
  // Multi-query results
  results: SqlResultEntry[];
  activeResultIdx: number;
}

export const sqlTabState = writable<Map<number, SqlTabData>>(new Map());
// Track which SQL tabs have unsaved result edits
export const sqlPendingChanges = writable<Set<number>>(new Set());

export function setSqlPending(tabId: number, hasPending: boolean) {
  sqlPendingChanges.update(s => {
    const next = new Set(s);
    if (hasPending) next.add(tabId);
    else next.delete(tabId);
    return next;
  });
}

const defaultSqlTabData = (): SqlTabData => ({
  query: '', result: null, error: null, loading: false,
  database: get(selectedDatabase) || '',
  results: [], activeResultIdx: 0,
});

export function getSqlTabData(tabId: number): SqlTabData {
  const map = get(sqlTabState);
  return map.get(tabId) ?? defaultSqlTabData();
}

export function setSqlTabData(tabId: number, data: Partial<SqlTabData>) {
  sqlTabState.update(m => {
    const next = new Map(m);
    const existing = m.get(tabId) ?? defaultSqlTabData();
    next.set(tabId, { ...existing, ...data });
    return next;
  });
}

export function clearSqlTabData(tabId: number) {
  sqlTabState.update(m => {
    const next = new Map(m);
    next.delete(tabId);
    return next;
  });
}

export function initSqlTab(tabId: number) {
  const db = get(selectedDatabase) || '';
  setSqlTabData(tabId, { query: '', result: null, error: null, loading: false, database: db, results: [], activeResultIdx: 0 });
}

export async function handleSqlConnectionSave(config: SqlConnectionConfig) {
  const editing = get(editingSqlConnection);
  if (editing) {
    const updated = await sqlCmd.sqlUpdateSavedConnection(editing.id, config);
    connections.update(c => c.map(x => x.id === editing.id ? updated : x));
  } else {
    const conn = await saveConnection(config);
    // Auto-connect after saving new connection
    try {
      await connectToDb(conn.id);
    } catch {
      // Connection saved but couldn't auto-connect
    }
  }
  showSqlConnectionDialog.set(false);
  editingSqlConnection.set(null);
}

export const activeConnection = derived(
  [connections, activeConnectionId],
  ([$connections, $activeConnectionId]) =>
    $connections.find(c => c.id === $activeConnectionId) ?? null
);

export async function loadConnections() {
  try {
    const data = await sqlCmd.sqlListSavedConnections();
    connections.set(data);
  } catch (err) {
    console.error('Failed to load SQL connections:', err);
  }
}

export async function saveConnection(config: SqlConnectionConfig): Promise<SqlConnection> {
  const conn = await sqlCmd.sqlSaveConnection(config);
  connections.update(c => [...c, conn]);
  activeConnectionId.set(conn.id);
  return conn;
}

export async function updateConnection(id: string, config: SqlConnectionConfig): Promise<SqlConnection> {
  const updated = await sqlCmd.sqlUpdateSavedConnection(id, config);
  connections.update(c => c.map(x => x.id === id ? updated : x));
  return updated;
}

export async function deleteConnection(id: string) {
  await sqlCmd.sqlDeleteSavedConnection(id);
  connections.update(c => c.filter(x => x.id !== id));
  if (get(activeConnectionId) === id) {
    activeConnectionId.set(null);
  }
  connectedIds.update(s => {
    const next = new Set(s);
    next.delete(id);
    return next;
  });
  // Clear cached data
  connectionDatabases.update(m => {
    const next = new Map(m);
    next.delete(id);
    return next;
  });
}

export async function connectToDb(connectionId: string) {
  const allConns = get(connections);
  const conn = allConns.find(c => c.id === connectionId);
  if (!conn) throw new Error('Connection not found');
  const liveId = await sqlCmd.sqlConnect({
    name: conn.name,
    driver: conn.driver as any,
    host: conn.host,
    port: conn.port,
    database: conn.databaseName,
    username: conn.username,
    password: conn.password,
    ssl: !!conn.ssl,
    // Forward the saved SSH profile so the backend opens the tunnel.
    // Without this the rewritten host/port (127.0.0.1:<bound>) is never
    // applied and the driver dials the original host directly.
    sshProfileId: conn.sshProfileId ?? null,
  });
  activeConnectionId.set(connectionId);
  connectedIds.update(s => {
    const next = new Set(s);
    next.add(connectionId);
    return next;
  });
  liveConnectionIds.update(m => ({ ...m, [connectionId]: liveId }));

  // Auto-expand and load databases
  expandedConnectionId.set(connectionId);
  try {
    const dbs = await sqlCmd.sqlListDatabases(liveId);
    connectionDatabases.update(m => {
      const next = new Map(m);
      next.set(connectionId, dbs);
      return next;
    });
    // Auto-select the configured database or first one
    if (dbs.length > 0) {
      const dbName = conn.databaseName || dbs[0];
      selectedDatabase.set(dbName);
      // Load tables for selected database
      await loadTablesForDb(connectionId, dbName);
    }
  } catch {
    // databases load failed — non-fatal
  }

  return liveId;
}

export async function loadTablesForDb(connectionId: string, database: string, schema?: string) {
  // For per-database loading, try to get a database-specific connection first
  let liveId = getDbLiveId(connectionId, database);
  if (!liveId) {
    // Fall back to the instance connection (works for initial database)
    liveId = getLiveId(connectionId);
  }
  if (!liveId) return;
  const key = schema ? `${connectionId}:${database}:${schema}` : `${connectionId}:${database}`;
  try {
    const tables = await sqlCmd.sqlListTables(liveId, database, schema);
    databaseTables.update(m => {
      const next = new Map(m);
      next.set(key, tables);
      return next;
    });
  } catch {
    databaseTables.update(m => {
      const next = new Map(m);
      next.set(key, []);
      return next;
    });
  }
}

export function getLiveId(savedId: string): string | null {
  return get(liveConnectionIds)[savedId] ?? null;
}

export function getDbLiveId(savedConnId: string, database: string): string | null {
  const key = `${savedConnId}:${database}`;
  return get(dbLiveConnections)[key] ?? null;
}

export async function connectToDatabase(savedConnId: string, database: string): Promise<string> {
  const key = `${savedConnId}:${database}`;
  // Return existing connection if already connected
  const existing = get(dbLiveConnections)[key];
  if (existing) return existing;

  const allConns = get(connections);
  const conn = allConns.find(c => c.id === savedConnId);
  if (!conn) throw new Error('Connection not found');

  const config = {
    name: conn.name,
    driver: conn.driver as any,
    host: conn.host,
    port: conn.port,
    database: conn.databaseName,
    username: conn.username,
    password: conn.password,
    ssl: !!conn.ssl,
    // Forward the saved SSH profile so per-database pools tunnel too.
    // The nav-expand path (loading databases/schemas/tables) lands here.
    sshProfileId: conn.sshProfileId ?? null,
  };

  // Use savedId:dbName as the pool key — same format as AI's ensure_pool
  const poolId = await sqlCmd.sqlConnectDatabase(config, database, key);
  dbLiveConnections.update(m => ({ ...m, [key]: poolId }));
  return poolId;
}

// --- SQL Script persistence ---

export async function loadSqlScripts() {
  try {
    const data = await sqlCmd.sqlListScripts();
    sqlScripts.set(data);
  } catch (err) {
    console.error('Failed to load SQL scripts:', err);
  }
}

export async function saveSqlScript(name: string, connectionId: string | null, databaseName: string, query: string): Promise<SqlScript> {
  const script = await sqlCmd.sqlSaveScript(name, connectionId, databaseName, query);
  sqlScripts.update(s => [...s, script]);
  return script;
}

export async function updateSqlScript(id: string, name: string, query: string, databaseName?: string): Promise<SqlScript> {
  const updated = await sqlCmd.sqlUpdateScript(id, name, query, databaseName);
  sqlScripts.update(s => s.map(x => x.id === id ? updated : x));
  return updated;
}

export async function deleteSqlScript(id: string) {
  await sqlCmd.sqlDeleteScript(id);
  sqlScripts.update(s => s.filter(x => x.id !== id));
}

export async function disconnectFromDb(connectionId: string) {
  const liveId = getLiveId(connectionId);
  if (liveId) {
    await sqlCmd.sqlDisconnect(liveId);
  }
  // Disconnect all per-database connections for this saved connection
  const dbConns = get(dbLiveConnections);
  const prefix = `${connectionId}:`;
  for (const [key, dbLiveId] of Object.entries(dbConns)) {
    if (key.startsWith(prefix)) {
      try { await sqlCmd.sqlDisconnect(dbLiveId); } catch { /* ignore */ }
    }
  }
  dbLiveConnections.update(m => {
    const next = { ...m };
    for (const key of Object.keys(next)) {
      if (key.startsWith(prefix)) delete next[key];
    }
    return next;
  });
  connectedIds.update(s => {
    const next = new Set(s);
    next.delete(connectionId);
    return next;
  });
  liveConnectionIds.update(m => {
    const next = { ...m };
    delete next[connectionId];
    return next;
  });
  // Clear cached data for this connection
  connectionDatabases.update(m => {
    const next = new Map(m);
    next.delete(connectionId);
    return next;
  });
  // Clear expanded state if this was expanded
  if (get(expandedConnectionId) === connectionId) {
    expandedConnectionId.set(null);
  }
  selectedDatabase.set('');
}

// AI execution trigger — AI writes query + connection info, SqlPanel auto-executes
export interface AiSqlExecution {
  query: string;
  connectionId: string;
  database: string;
}
export const aiExecuteQuery = writable<AiSqlExecution | null>(null);

export function triggerAiSqlExecution(query: string, connectionId: string, database: string) {
  aiExecuteQuery.set({ query, connectionId, database });
}
