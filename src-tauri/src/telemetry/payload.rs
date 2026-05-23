// Payload assembly + bucketing.
//
// All counts get translated to bucket strings here. The raw integers
// never leave this file — `assemble()` returns a struct that's already
// safe to serialize over the wire.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::telemetry::counters::{mode_name, DrainResult, FEATURE_KEYS};
use crate::telemetry::device::DeviceFingerprint;

#[derive(Debug, Serialize)]
pub struct HeartbeatPayload {
    pub device_id: String,
    pub app_version: &'static str,
    pub os: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    pub arch: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    pub modes_active: String,
    pub features: BTreeMap<&'static str, &'static str>,
    pub errors: BTreeMap<&'static str, &'static str>,
    pub db_buckets: BTreeMap<&'static str, &'static str>,
}

pub fn assemble(
    device: DeviceFingerprint,
    drained: DrainResult,
    db_counts: Vec<(&'static str, u64)>,
) -> HeartbeatPayload {
    let mut features = BTreeMap::new();
    for (k, v) in drained.features {
        // Sanity guard: only emit known keys. `drain()` only ever
        // returns keys from FEATURE_KEYS so this is belt + braces.
        if FEATURE_KEYS.contains(&k) {
            features.insert(k, bucketize(v));
        }
    }

    let mut errors = BTreeMap::new();
    for (k, v) in drained.errors {
        errors.insert(k, bucketize(v));
    }

    let mut db_buckets = BTreeMap::new();
    for (k, v) in db_counts {
        db_buckets.insert(k, bucketize(v));
    }

    let modes_active = expand_modes(drained.modes_bits);

    HeartbeatPayload {
        device_id: device.device_id,
        app_version: device.app_version,
        os: device.os,
        os_version: device.os_version,
        arch: device.arch,
        locale: device.locale,
        theme: device.theme,
        modes_active,
        features,
        errors,
        db_buckets,
    }
}

/// Map raw count → bucket label. Five buckets total. The "0" bucket
/// label exists for completeness but is never emitted — callers
/// filter zero counts out before they get here (see counters::drain).
pub fn bucketize(n: u64) -> &'static str {
    match n {
        0 => "0",
        1..=10 => "1-10",
        11..=100 => "11-100",
        101..=1000 => "101-1k",
        _ => "1k+",
    }
}

fn expand_modes(bits: u32) -> String {
    // Iterate the 7 known bits; collect names of set ones. Stable
    // ordering since we iterate in the declaration order of the
    // constants.
    use crate::telemetry::counters::{
        MODE_AGENT, MODE_EXPLORER, MODE_NOSQL, MODE_REST, MODE_SQL, MODE_SSH, MODE_WORKSPACE,
    };
    let mut names: Vec<&'static str> = Vec::with_capacity(7);
    for bit in [
        MODE_REST,
        MODE_SQL,
        MODE_NOSQL,
        MODE_SSH,
        MODE_AGENT,
        MODE_EXPLORER,
        MODE_WORKSPACE,
    ] {
        if bits & bit != 0 {
            let n = mode_name(bit);
            if !n.is_empty() {
                names.push(n);
            }
        }
    }
    names.join(",")
}

// ── DB row count buckets ───────────────────────────────────────────
//
// Five cheap COUNT(*) queries at flush time. Cheap because:
//   • The flush runs at most twice per 24h (scheduled + shutdown).
//   • Each table is small (saved REST requests, SSH profiles, etc.).
//   • SQLite COUNT is O(rows) but for these table sizes it's <1ms.

pub async fn collect_db_buckets(pool: &sqlx::SqlitePool) -> Vec<(&'static str, u64)> {
    let queries: &[(&'static str, &'static str)] = &[
        ("db.rest_requests", "SELECT COUNT(*) FROM requests"),
        ("db.ssh_profiles",  "SELECT COUNT(*) FROM ssh_profiles"),
        ("db.workspaces",    "SELECT COUNT(*) FROM workspaces"),
        ("db.coworkers",     "SELECT COUNT(*) FROM workspace_coworkers"),
        (
            "db.agent_sessions_30d",
            "SELECT COUNT(*) FROM agent_sessions \
             WHERE last_used_at >= datetime('now','-30 days')",
        ),
    ];

    let mut out = Vec::with_capacity(queries.len());
    for (key, sql) in queries {
        let count: (i64,) = match sqlx::query_as(sql).fetch_one(pool).await {
            Ok(c) => c,
            // If any table is missing (migration order, dev DB, etc.)
            // we just skip that bucket. Better to under-report than to
            // fail the whole flush.
            Err(_) => continue,
        };
        let v = count.0.max(0) as u64;
        if v > 0 {
            out.push((*key, v));
        }
    }
    out
}
