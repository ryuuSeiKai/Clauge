// Telemetry — append-only daily heartbeats.
//
// Design rules (the user signed off explicitly):
//   • Zero app impact. Hot path is `bump(key)` — a single atomic add on
//     a globally-shared `OnceCell<HashMap<&'static str, AtomicU64>>`.
//     No locks, no allocations, no I/O. Order is Relaxed because we're
//     counting, not synchronising with anything.
//   • Counters never accumulate beyond one 24h window. The scheduler
//     atomically swaps to 0 at flush time. If the POST fails the value
//     is lost; we don't retry. The next 24h window covers the gap.
//   • Counts are bucketed BEFORE leaving the device — wire format
//     never contains raw integers. Privacy guarantee at the source.
//   • No raw text fields. URLs, hosts, file paths, query bodies,
//     command lines, error messages — none of it lives in this module.
//   • Opt-out via `settings.telemetry_optout = "true"`. When opted out,
//     the scheduler skips the POST entirely and resets counters.
//
// Two cohorts share the same payload shape: when the user is logged in
// the bearer token is attached and the worker writes `user_id`; when
// they're not, the row lands as anonymous (NULL user_id). The local
// `device_id` is stable across login state — it's tied to the install,
// not the account.

pub mod counters;
pub mod device;
pub mod payload;
pub mod scheduler;

pub use counters::{bump, touch_mode, FEATURE_KEYS, ERROR_KEYS, MODE_AGENT, MODE_EXPLORER, MODE_NOSQL, MODE_REST, MODE_SQL, MODE_SSH, MODE_WORKSPACE};
pub use scheduler::spawn as spawn_scheduler;

/// Frontend-callable bump. Mirrors the Rust `bump()` but accepts an
/// owned String from the IPC boundary and only forwards keys that
/// match the compiled-in allowlist. Unknown keys are silently dropped
/// — a frontend typo can never grow the payload schema.
#[tauri::command]
pub fn telemetry_bump(key: String) {
    // Match against the static FEATURE_KEYS / ERROR_KEYS slices so we
    // can hand a &'static str to the counter registry. The .iter()
    // .find() is O(20) which is fine for IPC-rate calls.
    if let Some(&k) = FEATURE_KEYS.iter().find(|s| **s == key) {
        bump(k);
        return;
    }
    if let Some(&k) = ERROR_KEYS.iter().find(|s| **s == key) {
        bump(k);
    }
}
