// Counter registry — fixed at compile time so the hot path is a
// HashMap lookup with no allocation. Each counter is an AtomicU64;
// `bump(key)` is one `fetch_add(1, Relaxed)`.
//
// Why a HashMap and not a const array indexed by an enum:
//   • Bump sites pass a `&'static str` literal — no compile-time
//     coupling between the call site and the registry. Easier to add
//     new keys later, lower risk of "added a key but forgot to extend
//     the array" bugs.
//   • The lookup runs once per user action (REST execute, SSH connect,
//     etc.). User-facing actions happen at human speed; the lookup
//     cost is invisible.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::OnceLock;

// ── Feature counter keys ───────────────────────────────────────────
//
// Drives both the registry at boot AND the JSON payload at flush time.
// Adding a new key = add it here; bump site has to match the literal.
// Keep this list short — every new key permanently grows the payload
// envelope. 16 keys covers all 7 modes' headline actions.
pub const FEATURE_KEYS: &[&str] = &[
    "rest.execute",          // REST request sent (saved or unsaved)
    "rest.curl_paste",       // cURL command pasted into URL bar
    "rest.curl_export",      // "Copy as cURL" clicked
    "rest.format_json",      // JSON Format button used
    "rest.env_var_used",     // {{var}} resolved at request time
    "sql.execute",           // SQL query run
    "nosql.execute",         // Mongo / Redis op
    "ssh.connect",           // SSH session opened (handshake done)
    "ssh.execute_shell",     // AI execute_shell tool invocation
    "explorer.transfer",     // file transfer (download or upload)
    "agent.spawn",           // Claude CLI session started
    "agent.git_commit",      // git commit via the agent panel
    "workspace.card_create", // kanban card created
    "workspace.coworker_use",// @coworker mention sent
    "ai.chat",               // AI panel chat message sent
    "ai.tool_call",          // AI tool invocation
];

// ── Error class counter keys ───────────────────────────────────────
//
// We count CATEGORIES, never store messages. The classification is the
// signal — "ssh connect timed out 12 times this week" tells product
// what to investigate.
pub const ERROR_KEYS: &[&str] = &[
    "err.ssh_timeout",       // russh connect / handshake timed out
    "err.http_5xx",          // user's API target returned 5xx in REST
    "err.sql_connect_fail",  // user's SQL driver couldn't connect
    "err.api_5xx",           // OUR worker returned 5xx (sync/ai/billing)
];

// ── Mode bitmask ───────────────────────────────────────────────────
//
// Which modes did the user actually touch during the 24h window?
// Single AtomicU32; the scheduler reads + clears at flush. Order
// here is for serialization only — the bits themselves are stable.
pub const MODE_REST:      u32 = 1 << 0;
pub const MODE_SQL:       u32 = 1 << 1;
pub const MODE_NOSQL:     u32 = 1 << 2;
pub const MODE_SSH:       u32 = 1 << 3;
pub const MODE_AGENT:     u32 = 1 << 4;
pub const MODE_EXPLORER:  u32 = 1 << 5;
pub const MODE_WORKSPACE: u32 = 1 << 6;

pub fn mode_name(bit: u32) -> &'static str {
    match bit {
        MODE_REST => "rest",
        MODE_SQL => "sql",
        MODE_NOSQL => "nosql",
        MODE_SSH => "ssh",
        MODE_AGENT => "agent",
        MODE_EXPLORER => "explorer",
        MODE_WORKSPACE => "workspace",
        _ => "",
    }
}

// ── Storage ────────────────────────────────────────────────────────

static COUNTERS: OnceLock<HashMap<&'static str, AtomicU64>> = OnceLock::new();
static MODES_ACTIVE: AtomicU32 = AtomicU32::new(0);

fn registry() -> &'static HashMap<&'static str, AtomicU64> {
    COUNTERS.get_or_init(|| {
        let mut m = HashMap::with_capacity(FEATURE_KEYS.len() + ERROR_KEYS.len());
        for k in FEATURE_KEYS {
            m.insert(*k, AtomicU64::new(0));
        }
        for k in ERROR_KEYS {
            m.insert(*k, AtomicU64::new(0));
        }
        m
    })
}

// ── Public API ─────────────────────────────────────────────────────

/// Hot-path bump. Safe to call from anywhere, including sync contexts.
/// Unknown keys are silently dropped (caller passed a typo / removed
/// key) — we don't want a typo to crash the user's action.
///
/// Also marks the corresponding mode as active for free, so callers
/// don't need to remember a separate `touch_mode()` call. The dotted
/// prefix on the key identifies the mode.
pub fn bump(key: &'static str) {
    if let Some(c) = registry().get(key) {
        c.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(bit) = mode_for_key(key) {
        MODES_ACTIVE.fetch_or(bit, Ordering::Relaxed);
    }
}

fn mode_for_key(key: &str) -> Option<u32> {
    // The 'ai.*' family doesn't bind to a single mode — AI chat can
    // run from any mode's panel. Skip mode-tagging for those.
    if let Some(dot) = key.find('.') {
        return match &key[..dot] {
            "rest" => Some(MODE_REST),
            "sql" => Some(MODE_SQL),
            "nosql" => Some(MODE_NOSQL),
            "ssh" => Some(MODE_SSH),
            "agent" => Some(MODE_AGENT),
            "explorer" => Some(MODE_EXPLORER),
            "workspace" => Some(MODE_WORKSPACE),
            _ => None,
        };
    }
    None
}

/// Mark a mode as touched in the current 24h window.
pub fn touch_mode(mode: u32) {
    MODES_ACTIVE.fetch_or(mode, Ordering::Relaxed);
}

/// Atomically swap each counter to 0 and return the prior values.
/// Called by the scheduler at flush time.
pub fn drain() -> DrainResult {
    let mut features = Vec::new();
    let mut errors = Vec::new();
    let reg = registry();
    for k in FEATURE_KEYS {
        if let Some(c) = reg.get(k) {
            let v = c.swap(0, Ordering::Relaxed);
            if v > 0 {
                features.push((*k, v));
            }
        }
    }
    for k in ERROR_KEYS {
        if let Some(c) = reg.get(k) {
            let v = c.swap(0, Ordering::Relaxed);
            if v > 0 {
                errors.push((*k, v));
            }
        }
    }
    let modes_bits = MODES_ACTIVE.swap(0, Ordering::Relaxed);
    DrainResult {
        features,
        errors,
        modes_bits,
    }
}

/// If a flush fails after `drain()` already consumed the counters,
/// the caller restores them via this. We accept a possible race
/// (concurrent bumps between drain and restore) — the bucketing tier
/// absorbs small inaccuracies, so this is fine. Critically we DO NOT
/// store-then-restore for raw integers — we just add the drained
/// counts back, so concurrent bumps are preserved.
pub fn restore(drained: &DrainResult) {
    let reg = registry();
    for (k, v) in &drained.features {
        if let Some(c) = reg.get(k) {
            c.fetch_add(*v, Ordering::Relaxed);
        }
    }
    for (k, v) in &drained.errors {
        if let Some(c) = reg.get(k) {
            c.fetch_add(*v, Ordering::Relaxed);
        }
    }
    MODES_ACTIVE.fetch_or(drained.modes_bits, Ordering::Relaxed);
}

#[derive(Debug, Default)]
pub struct DrainResult {
    pub features: Vec<(&'static str, u64)>,
    pub errors: Vec<(&'static str, u64)>,
    pub modes_bits: u32,
}

impl DrainResult {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.errors.is_empty() && self.modes_bits == 0
    }
}
