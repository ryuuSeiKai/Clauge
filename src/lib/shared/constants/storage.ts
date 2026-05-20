// Centralized localStorage keys + thin typed accessors.
// All app-level keys follow the `clauge_*` convention.
//
// When adding a new key:
//   1. Add it to STORAGE_KEYS below.
//   2. If you want a typed accessor, add a `get*`/`set*` helper.
//   3. Never inline `localStorage.getItem('clauge_…')` in components.

export const STORAGE_KEYS = {
  ACTIVE_ENV_ID: 'clauge_active_env_id',
  AI_CHAT_HISTORY: 'clauge_ai_chat_history',
  AI_PANEL_WIDTH: 'clauge_ai_panel_width',
  GITHUB_AVATAR: 'clauge_github_avatar',
  HAS_SYNCED: 'clauge_has_synced',
  LAST_MODE: 'clauge_last_mode',
  LAST_SEEN_VERSION: 'clauge_last_seen_version',
  LAST_SYNCED_AT: 'clauge_last_synced_at',
  NAV_PINNED: 'clauge_nav_pinned',
  REQUEST_ENV_OVERRIDES: 'clauge_request_env_overrides',
  SQL_ROW_LIMIT: 'clauge_sql_row_limit',
  SSH_AUTO_RUN: 'clauge_ssh_auto_run',
  UPDATE_CHANNEL: 'clauge_update_channel',
} as const;

// Legacy keys we migrate away from on first read.
const LEGACY_SQL_ROW_LIMIT = 'sqlRowLimit';

const hasLs = (): boolean => typeof localStorage !== 'undefined';

// --- nav pinned (boolean) ---

export function getNavPinned(): boolean {
  if (!hasLs()) return true;
  // Default true: nav is pinned unless explicitly turned off.
  return localStorage.getItem(STORAGE_KEYS.NAV_PINNED) !== 'false';
}

export function setNavPinned(value: boolean): void {
  if (!hasLs()) return;
  localStorage.setItem(STORAGE_KEYS.NAV_PINNED, String(value));
}

// --- ssh auto-run (boolean) ---

export function getSshAutoRun(): boolean {
  if (!hasLs()) return false;
  return localStorage.getItem(STORAGE_KEYS.SSH_AUTO_RUN) === 'true';
}

export function setSshAutoRun(value: boolean): void {
  if (!hasLs()) return;
  localStorage.setItem(STORAGE_KEYS.SSH_AUTO_RUN, String(value));
}

// --- AI panel width (number) ---

export function getAiPanelWidth(fallback: number): number {
  if (!hasLs()) return fallback;
  const raw = localStorage.getItem(STORAGE_KEYS.AI_PANEL_WIDTH);
  const parsed = raw ? parseInt(raw, 10) : NaN;
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function setAiPanelWidth(width: number): void {
  if (!hasLs()) return;
  localStorage.setItem(STORAGE_KEYS.AI_PANEL_WIDTH, String(width));
}

// --- SQL row limit (number) — with one-time migration from legacy `sqlRowLimit` ---

export function getSqlRowLimit(fallback: number): number {
  if (!hasLs()) return fallback;
  let raw = localStorage.getItem(STORAGE_KEYS.SQL_ROW_LIMIT);
  if (raw === null) {
    // Migrate from pre-rename key if present.
    const legacy = localStorage.getItem(LEGACY_SQL_ROW_LIMIT);
    if (legacy !== null) {
      localStorage.setItem(STORAGE_KEYS.SQL_ROW_LIMIT, legacy);
      localStorage.removeItem(LEGACY_SQL_ROW_LIMIT);
      raw = legacy;
    }
  }
  if (raw === null) return fallback;
  const parsed = parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

export function setSqlRowLimit(value: number): void {
  if (!hasLs()) return;
  localStorage.setItem(STORAGE_KEYS.SQL_ROW_LIMIT, String(value));
}
