import { writable, derived, get } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type {
  SqlConnection,
  SqlConnectionConfig,
  TableInfo,
  SqlQueryResult,
  SqlResultEntry,
  SqlScript,
  Binding,
  InFlight,
  PoolState,
} from './types';
import * as sqlCmd from './commands';
import { getSqlRowLimit, setSqlRowLimit } from '$lib/shared/constants/storage';
import { tabs, activeTabId } from '$lib/shared/stores/tabs';

// --- Saved connection profiles ------------------------------------------------

export const connections = writable<SqlConnection[]>([]);

// Sidebar UX only — what row is currently expanded. These NO LONGER drive
// query execution; that's the per-tab `binding` field.
export const activeConnectionId = writable<string | null>(null);
export const expandedConnectionId = writable<string | null>(null);

// Cached metadata per `(conn, db)` key — fetched lazily by SqlNav / SqlPanel.
export const connectionDatabases = writable<Map<string, string[]>>(new Map());
export const databaseTables = writable<Map<string, TableInfo[]>>(new Map());

// --- Pool state (keyed by `${connId}:${db}`) ---------------------------------

export const poolStates = writable<Map<string, PoolState>>(new Map());
export const poolErrors = writable<Map<string, string>>(new Map());
/** `${connId}:${db}` → expiry timestamp (ms). Drives the amber
 *  "reconnecting…" badge for ~2s after a dead-pool rebuild fires. */
export const reconnectingFlash = writable<Map<string, number>>(new Map());

export function poolKey(connId: string, database: string): string {
  return `${connId}:${database}`;
}

function setPoolState(key: string, s: PoolState) {
  poolStates.update((m) => {
    const n = new Map(m);
    if (s === 'idle') n.delete(key);
    else n.set(key, s);
    return n;
  });
}

function setPoolError(key: string, msg: string | null) {
  poolErrors.update((m) => {
    const n = new Map(m);
    if (msg) n.set(key, msg);
    else n.delete(key);
    return n;
  });
}

export function getPoolState(connId: string, db: string): PoolState {
  return get(poolStates).get(poolKey(connId, db)) ?? 'idle';
}

// One Promise per in-flight ensure call so 10 simultaneous triggers
// (nav click, tab effect, Run press) collapse to a single backend handshake.
const inflightEnsures = new Map<string, Promise<void>>();

/**
 * Idempotent, deduplicated pool opener. Sets the pool state to
 * `connecting` for the duration; flips to `connected` on success or
 * `error` on failure. Repeated calls for the same `(connId, db)` while
 * one is in flight share the same Promise.
 */
export function ensureConnected(connId: string, db: string): Promise<void> {
  const key = poolKey(connId, db);
  if (get(poolStates).get(key) === 'connected') return Promise.resolve();
  const existing = inflightEnsures.get(key);
  if (existing) return existing;

  setPoolState(key, 'connecting');
  setPoolError(key, null);
  const p = (async () => {
    try {
      await sqlCmd.sqlEnsurePool(connId, db);
      setPoolState(key, 'connected');
    } catch (e: any) {
      setPoolState(key, 'error');
      setPoolError(key, String(e));
      throw e;
    } finally {
      inflightEnsures.delete(key);
    }
  })();
  inflightEnsures.set(key, p);
  return p;
}

export async function disconnectConnection(connId: string) {
  await sqlCmd.sqlDisconnectConnection(connId);
  // Clear every pool entry under this connection.
  const prefix = `${connId}:`;
  poolStates.update((m) => {
    const n = new Map(m);
    for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
    return n;
  });
  poolErrors.update((m) => {
    const n = new Map(m);
    for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
    return n;
  });
  connectionDatabases.update((m) => {
    const n = new Map(m);
    n.delete(connId);
    return n;
  });
  databaseTables.update((m) => {
    const n = new Map(m);
    for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
    return n;
  });
}

// --- Tab state ---------------------------------------------------------------

export interface SqlTabData {
  binding: Binding | null;
  query: string;
  result: SqlQueryResult | null;
  error: string | null;
  inFlight: InFlight | null;
  /** Multi-result tabs (one query → one result entry; multi-statement → many). */
  results: SqlResultEntry[];
  activeResultIdx: number;
}

export const sqlTabState = writable<Map<number, SqlTabData>>(new Map());

const defaultSqlTabData = (): SqlTabData => ({
  binding: null,
  query: '',
  result: null,
  error: null,
  inFlight: null,
  results: [],
  activeResultIdx: 0,
});

export function getSqlTabData(tabId: number): SqlTabData {
  return get(sqlTabState).get(tabId) ?? defaultSqlTabData();
}

export function setSqlTabData(tabId: number, patch: Partial<SqlTabData>) {
  sqlTabState.update((m) => {
    const n = new Map(m);
    const existing = m.get(tabId) ?? defaultSqlTabData();
    n.set(tabId, { ...existing, ...patch });
    return n;
  });
}

export function clearSqlTabData(tabId: number) {
  sqlTabState.update((m) => {
    const n = new Map(m);
    n.delete(tabId);
    return n;
  });
}

export function initSqlTab(tabId: number) {
  setSqlTabData(tabId, defaultSqlTabData());
}

/** Track which SQL tabs have unsaved result edits (cell-edit feature). */
export const sqlPendingChanges = writable<Set<number>>(new Set());
export function setSqlPending(tabId: number, hasPending: boolean) {
  sqlPendingChanges.update((s) => {
    const n = new Set(s);
    if (hasPending) n.add(tabId);
    else n.delete(tabId);
    return n;
  });
}

// --- Saved-connection management --------------------------------------------

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
  connections.update((c) => [...c, conn]);
  activeConnectionId.set(conn.id);
  return conn;
}

export async function updateConnection(id: string, config: SqlConnectionConfig): Promise<SqlConnection> {
  const updated = await sqlCmd.sqlUpdateSavedConnection(id, config);
  connections.update((c) => c.map((x) => (x.id === id ? updated : x)));
  return updated;
}

export async function deleteConnection(id: string) {
  await sqlCmd.sqlDeleteSavedConnection(id);
  await disconnectConnection(id);
  connections.update((c) => c.filter((x) => x.id !== id));
  if (get(activeConnectionId) === id) activeConnectionId.set(null);
  // Any tab bound to this connection falls back to the first remaining
  // connection's default DB. If none remain, binding becomes null and the
  // tab shows the "Pick a connection" banner.
  const remaining = get(connections);
  const fallback = remaining[0] ?? null;
  sqlTabState.update((m) => {
    const n = new Map(m);
    let changed = false;
    for (const [tabId, tab] of n) {
      if (tab.binding?.connectionId === id) {
        const next: Binding | null = fallback
          ? { connectionId: fallback.id, database: fallback.databaseName }
          : null;
        n.set(tabId, { ...tab, binding: next, inFlight: null });
        changed = true;
      }
    }
    return changed ? n : m;
  });
}

// Bumped whenever a saved connection's config is updated. Components that
// keep their own per-connection caches (e.g. SqlNav's schema/table/column
// maps) watch this map and clear the affected entries — without it, an edit
// to e.g. a SQLite file path would still surface the old file's tree.
export const sqlConnectionConfigVersion = writable<Map<string, number>>(new Map());
function bumpConnectionConfigVersion(id: string) {
  sqlConnectionConfigVersion.update((m) => {
    const n = new Map(m);
    n.set(id, (n.get(id) ?? 0) + 1);
    return n;
  });
}

export async function handleSqlConnectionSave(config: SqlConnectionConfig) {
  const editing = get(editingSqlConnection);
  if (editing) {
    const updated = await sqlCmd.sqlUpdateSavedConnection(editing.id, config);
    connections.update((c) => c.map((x) => (x.id === editing.id ? updated : x)));
    // Backend already drops its pools on update — also clear the frontend
    // store-level caches so the UI doesn't show stale databases/tables, and
    // notify components with local caches to invalidate via the version bump.
    connectionDatabases.update((m) => { const n = new Map(m); n.delete(editing.id); return n; });
    databaseTables.update((m) => {
      const n = new Map(m);
      const prefix = `${editing.id}:`;
      for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
      return n;
    });
    bumpConnectionConfigVersion(editing.id);
  } else {
    const conn = await saveConnection(config);
    // Auto-bind the active tab to the newly saved connection's default DB.
    const tabId = get(activeTabId);
    if (tabId >= 0) {
      const tab = get(tabs).find((t) => t.id === tabId);
      if (tab?.mode === 'sql') {
        setSqlTabData(tabId, { binding: { connectionId: conn.id, database: conn.databaseName } });
      }
    }
  }
  showSqlConnectionDialog.set(false);
  editingSqlConnection.set(null);
}

// --- UI singletons (dialog open state, etc.) --------------------------------

export const showSqlConnectionDialog = writable(false);
export const editingSqlConnection = writable<SqlConnection | null>(null);
export const showSqlDisconnectConfirm = writable(false);
export const sqlDisconnectTarget = writable<SqlConnection | null>(null);

// Default row limit for SELECT queries (0 = no limit). `getSqlRowLimit`
// migrates the legacy `sqlRowLimit` key on first read.
const SQL_ROW_LIMIT_DEFAULT = 100;
export const sqlRowLimit = writable<number>(getSqlRowLimit(SQL_ROW_LIMIT_DEFAULT));
sqlRowLimit.subscribe((v) => setSqlRowLimit(v));

// --- Active-tab derived helpers ---------------------------------------------

export const activeSqlTabData = derived(
  [sqlTabState, activeTabId, tabs],
  ([$sqlTabState, $activeTabId, $tabs]) => {
    const tab = $tabs.find((t) => t.id === $activeTabId && t.mode === 'sql');
    if (!tab) return null;
    return $sqlTabState.get(tab.id) ?? defaultSqlTabData();
  },
);

export const activeBinding = derived(activeSqlTabData, (d) => d?.binding ?? null);

export const activeConnection = derived([connections, activeBinding], ([$connections, $binding]) =>
  $binding ? $connections.find((c) => c.id === $binding.connectionId) ?? null : null,
);

// --- Setting a tab's binding -------------------------------------------------

/**
 * Update a tab's `(connection, database)` selection. Fire-and-forget triggers
 * `ensureConnected` so the loader appears immediately. Failures surface via
 * `poolErrors` and the per-pool state turning `error` — the tab itself stays
 * bound (user can retry).
 */
export function setBinding(tabId: number, connId: string, database: string) {
  setSqlTabData(tabId, {
    binding: { connectionId: connId, database },
    inFlight: null,
    error: null,
  });
  ensureConnected(connId, database).catch(() => {
    /* surfaced via poolErrors */
  });
}

// --- Insert-from-nav / AI execute --------------------------------------------

export const insertQueryText = writable<string>('');

export function applyAiQuery(query: string) {
  insertQueryText.set(query);
}

export interface AiSqlExecution {
  query: string;
  connectionId: string;
  database: string;
}

export const aiExecuteQuery = writable<AiSqlExecution | null>(null);

/**
 * AI/MCP-triggered query. Reuses the active tab IFF its binding matches the
 * AI's target `(conn, db)`; otherwise opens a new tab. The target is always
 * bound to the AI's `(connId, database)` so multi-instance setups behave
 * predictably.
 */
export async function triggerAiSqlExecution(query: string, connectionId: string, database: string) {
  const { addTab } = await import('$lib/shared/stores/tabs');
  const activeId = get(activeTabId);
  const activeTab = get(tabs).find((t) => t.id === activeId && t.mode === 'sql');
  const activeData = activeTab ? get(sqlTabState).get(activeTab.id) : null;
  const matches =
    activeData?.binding?.connectionId === connectionId &&
    activeData?.binding?.database === database;
  const targetTab = activeTab && matches ? activeTab : addTab('AI Query', 'sql', null, 'var(--sql)');
  setSqlTabData(targetTab.id, {
    binding: { connectionId, database },
    query,
    results: [],
    activeResultIdx: 0,
    inFlight: null,
    error: null,
  });
  aiExecuteQuery.set({ query, connectionId, database });
}

// --- Cancel ------------------------------------------------------------------

export async function cancelQuery(tabId: number) {
  const data = get(sqlTabState).get(tabId);
  if (!data?.inFlight) return;
  await sqlCmd.sqlCancelQuery(data.inFlight.queryId);
}

// --- SQL script CRUD --------------------------------------------------------

export const sqlScripts = writable<SqlScript[]>([]);

export async function loadSqlScripts() {
  try {
    const data = await sqlCmd.sqlListScripts();
    sqlScripts.set(data);
  } catch (err) {
    console.error('Failed to load SQL scripts:', err);
  }
}

export async function saveSqlScript(
  name: string,
  connectionId: string | null,
  databaseName: string,
  query: string,
): Promise<SqlScript> {
  const script = await sqlCmd.sqlSaveScript(name, connectionId, databaseName, query);
  sqlScripts.update((s) => [...s, script]);
  return script;
}

export async function updateSqlScript(
  id: string,
  name: string,
  query: string,
  databaseName?: string,
  connectionId?: string,
): Promise<SqlScript> {
  const updated = await sqlCmd.sqlUpdateScript(id, name, query, databaseName, connectionId);
  sqlScripts.update((s) => s.map((x) => (x.id === id ? updated : x)));
  return updated;
}

export async function deleteSqlScript(id: string) {
  await sqlCmd.sqlDeleteScript(id);
  sqlScripts.update((s) => s.filter((x) => x.id !== id));
}

// --- Database/table metadata loaders ---------------------------------------

/** Load + cache the database list for a saved connection. Requires the
 *  default `(connId, conn.databaseName)` pool to be reachable. */
export async function loadDatabaseList(connId: string): Promise<string[]> {
  const conn = get(connections).find((c) => c.id === connId);
  if (!conn) return [];
  try {
    await ensureConnected(connId, conn.databaseName);
    const dbs = await sqlCmd.sqlListDatabases(connId, conn.databaseName);
    connectionDatabases.update((m) => {
      const n = new Map(m);
      n.set(connId, dbs);
      return n;
    });
    return dbs;
  } catch {
    return [];
  }
}

/** Refresh the table list for `(connId, database[, schema])`. Cached under
 *  `${connId}:${database}[:${schema}]` so SqlPanel + SqlNav share data. */
export async function loadTablesForDb(connId: string, database: string, schema?: string) {
  try {
    await ensureConnected(connId, database);
    const tables = await sqlCmd.sqlListTables(connId, database, schema);
    const key = schema ? `${connId}:${database}:${schema}` : `${connId}:${database}`;
    databaseTables.update((m) => {
      const n = new Map(m);
      n.set(key, tables);
      return n;
    });
  } catch {
    /* leave cache as-is */
  }
}

// --- Compatibility shims for sidebar/nav consumers --------------------------
//
// SqlNav was written against the previous globally-keyed model
// (`connectedIds: Set<savedId>`, `connectToDb`, `disconnectFromDb`,
// `getLiveId`, `getDbLiveId`, `connectToDatabase`, `dbLiveConnections`,
// `sqlConnectionStates`, `sqlConnectionErrors`, `resetSqlConnState`).
// These shims preserve the sidebar's click-handler logic while routing
// everything through the new per-`(connId, db)` pool keyspace. They are
// derived from `poolStates` / `poolErrors`; setters mutate those.

/** Saved-connection IDs that have at least one open pool. Used by nav
 *  for the "connected" indicator. */
export const connectedIds = derived(poolStates, ($poolStates) => {
  const set = new Set<string>();
  for (const [k, s] of $poolStates) {
    if (s === 'connected') {
      const idx = k.indexOf(':');
      if (idx > 0) set.add(k.slice(0, idx));
    }
  }
  return set;
});

/** Rolled-up state per saved-connection: connecting > error > connected > idle. */
export const sqlConnectionStates = derived(poolStates, ($poolStates) => {
  const m = new Map<string, PoolState>();
  for (const [k, s] of $poolStates) {
    const idx = k.indexOf(':');
    if (idx < 0) continue;
    const connId = k.slice(0, idx);
    const prev = m.get(connId);
    if (s === 'connecting') m.set(connId, 'connecting');
    else if (prev !== 'connecting' && s === 'error') m.set(connId, 'error');
    else if (prev !== 'connecting' && prev !== 'error' && s === 'connected') m.set(connId, 'connected');
    else if (!prev) m.set(connId, s);
  }
  return m;
});

export const sqlConnectionErrors = derived(poolErrors, ($poolErrors) => {
  const m = new Map<string, string>();
  for (const [k, msg] of $poolErrors) {
    const idx = k.indexOf(':');
    if (idx < 0) continue;
    m.set(k.slice(0, idx), msg);
  }
  return m;
});

export function resetSqlConnState(connId: string) {
  const prefix = `${connId}:`;
  poolStates.update((m) => {
    const n = new Map(m);
    for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
    return n;
  });
  poolErrors.update((m) => {
    const n = new Map(m);
    for (const k of [...n.keys()]) if (k.startsWith(prefix)) n.delete(k);
    return n;
  });
}

/** Legacy `connectToDb(connId)` — connect to the saved record's default
 *  database. Routes through `ensureConnected` and then performs the
 *  side-effects SqlNav relies on for the sidebar expansion: marks this
 *  connection as expanded and fetches the database list so the tree
 *  has children to render. Both side-effects are best-effort. */
export async function connectToDb(connId: string): Promise<string> {
  const conn = get(connections).find((c) => c.id === connId);
  if (!conn) throw new Error(`Saved connection record missing (id ${connId})`);
  await ensureConnected(conn.id, conn.databaseName);
  expandedConnectionId.set(connId);
  // Best-effort: populate the connection's database list so the sidebar
  // tree has rows to render. Already-cached lists short-circuit.
  if (!get(connectionDatabases).has(connId)) {
    loadDatabaseList(connId).catch(() => {
      /* surfaced via toast in caller */
    });
  }
  return poolKey(conn.id, conn.databaseName);
}

/** Legacy `connectToDatabase(connId, db)` — open the per-database pool.
 *  Returns the same `${connId}:${db}` key shape as before. */
export async function connectToDatabase(connId: string, db: string): Promise<string> {
  await ensureConnected(connId, db);
  return poolKey(connId, db);
}

/** Legacy `disconnectFromDb(connId)` — close every pool under this
 *  saved connection. */
export async function disconnectFromDb(connId: string): Promise<void> {
  await disconnectConnection(connId);
}

/** Legacy `getLiveId(connId)` — was a UUID; now returns the canonical
 *  `${connId}:${conn.databaseName}` if that pool is open, else null. */
export function getLiveId(connId: string): string | null {
  const conn = get(connections).find((c) => c.id === connId);
  if (!conn) return null;
  const k = poolKey(connId, conn.databaseName);
  return get(poolStates).get(k) === 'connected' ? k : null;
}

/** Legacy `getDbLiveId(connId, db)` — returns `${connId}:${db}` if that
 *  pool is open, else null. */
export function getDbLiveId(connId: string, db: string): string | null {
  const k = poolKey(connId, db);
  return get(poolStates).get(k) === 'connected' ? k : null;
}

/** Legacy `dbLiveConnections: Record<string, string>` — the keys ARE the
 *  pool-keys now, so we just project a key→key map for any connected pool. */
export const dbLiveConnections = derived(poolStates, ($poolStates) => {
  const m: Record<string, string> = {};
  for (const [k, s] of $poolStates) if (s === 'connected') m[k] = k;
  return m;
});

/** Legacy `liveConnectionIds: Record<savedId, string>` — defaults to the
 *  saved record's default-DB pool key. */
export const liveConnectionIds = derived(
  [poolStates, connections],
  ([$poolStates, $connections]) => {
    const m: Record<string, string> = {};
    for (const c of $connections) {
      const k = poolKey(c.id, c.databaseName);
      if ($poolStates.get(k) === 'connected') m[c.id] = k;
    }
    return m;
  },
);

/** Selected sidebar database — kept for nav UX state only. */
export const selectedDatabase = writable<string>('');

/** Is *any* pool open for the currently-active sidebar connection? Used
 *  by the Topbar disconnect indicator. */
export const sqlIsConnected = derived(
  [activeConnectionId, connectedIds],
  ([$activeConnectionId, $connectedIds]) =>
    $activeConnectionId ? $connectedIds.has($activeConnectionId) : false,
);

// --- Tauri event listeners (registered once at app boot) --------------------

let listenersRegistered = false;
export function registerSqlEventListeners() {
  if (listenersRegistered) return;
  listenersRegistered = true;
  listen<string>('sql:reconnecting', (e) => {
    const key = e.payload;
    const expires = Date.now() + 2000;
    reconnectingFlash.update((m) => {
      const n = new Map(m);
      n.set(key, expires);
      return n;
    });
    setTimeout(() => {
      reconnectingFlash.update((m) => {
        const n = new Map(m);
        const t = n.get(key);
        if (t && t <= Date.now()) n.delete(key);
        return n;
      });
    }, 2100);
  });
}
