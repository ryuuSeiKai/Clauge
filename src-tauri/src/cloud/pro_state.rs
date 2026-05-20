// Single in-memory authority for Pro entitlement state.
//
// Before: 8 independent code paths answered "is the user Pro?"
//   • cloud:plan SQLite setting (read by 1 non-cloud Rust call)
//   • cloud:credits_snapshot / cloud:sub_snapshot SQLite settings
//   • AuthState (tokens only)
//   • $cloudPlan Svelte store (4+ writers)
//   • $cloudCredits Svelte store (4 writers)
//   • $cloudSub Svelte store (3 writers)
//   • per-component gates (9 files)
//   • welcomeProPlanHint / postCheckoutVerifying transient stores
//
// And transitions fired only inside `cloud_get_status`'s Ok branch,
// keying off the OLD `cloud:plan` SQLite row vs the NEW response. That
// guard silently broke on sign-out: `auth::clear()` deleted the row, so
// the next `cloud_get_status` saw `old_plan = None` and never fired the
// Pro→Free downgrade hook.
//
// After: this manager owns the truth. It serializes all transitions
// through one mutator (`apply` / `clear`), runs hooks based on the
// in-memory diff (immune to the SQLite-key-wiped race), persists the
// snapshot in one place, and emits a single `cloud:pro-state` event.
//
// Commit 1 (this file): introduce the manager and route the Rust call
// sites through it. Frontend stays on its existing `cloudPlan`/
// `cloudCredits`/`cloudSub` stores — we keep writing the legacy SQLite
// keys so the existing snapshot-hydration path in +layout.svelte keeps
// working unchanged. Commit 2 migrates readers; Commit 3 consolidates.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::cloud::config::{
    SETTINGS_KEY_CREDITS_SNAPSHOT, SETTINGS_KEY_PLAN, SETTINGS_KEY_SUB_SNAPSHOT,
};
use crate::cloud::models::{CloudCredits, CloudEntitlements, CloudSubscription};
use crate::shared::repos::settings;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProState {
    /// "free" | "pro". Mirrors `users.plan` from the worker.
    #[serde(default = "default_plan")]
    pub plan: String,
    #[serde(default)]
    pub credits: Option<CloudCredits>,
    #[serde(default)]
    pub subscription: Option<CloudSubscription>,
}

fn default_plan() -> String {
    "free".to_string()
}

impl ProState {
    pub fn is_pro(&self) -> bool {
        self.plan == "pro"
    }

    pub fn free() -> Self {
        Self {
            plan: default_plan(),
            credits: None,
            subscription: None,
        }
    }
}

/// What initiated a state change. Hooks read this to decide whether to
/// run side-effects (BootHydrate is silent — pure restoration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// Restoring state from on-disk snapshot at app boot. Hooks DO NOT
    /// fire — the snapshot is optimistic, server will reconcile shortly.
    BootHydrate,
    /// Server-authoritative apply (cloud_get_status, cloud_exchange_code,
    /// link/unlink/update_profile, post-checkout poll). Hooks fire.
    Apply,
    /// Explicit clear (sign-out, NotAuthenticated, invalid_grant).
    /// Hooks fire so Pro→Free coworker disable runs even on sign-out —
    /// closes the bug where sign-out left 4 active coworkers visible.
    Clear,
}

pub struct ProStateManager {
    inner: RwLock<ProState>,
}

impl Default for ProStateManager {
    fn default() -> Self {
        Self {
            inner: RwLock::new(ProState::default()),
        }
    }
}

impl ProStateManager {
    pub fn current(&self) -> ProState {
        self.inner.read().clone()
    }

    pub fn is_pro(&self) -> bool {
        self.inner.read().is_pro()
    }

    /// Apply new state. Atomic swap of in-memory value, then hooks (if
    /// not BootHydrate), then snapshot persist, then event emit.
    pub async fn apply(
        &self,
        new: ProState,
        trigger: Trigger,
        app: &AppHandle,
        pool: &SqlitePool,
    ) -> Result<(), String> {
        let old = {
            let mut guard = self.inner.write();
            let old = guard.clone();
            *guard = new.clone();
            old
        };
        if trigger != Trigger::BootHydrate {
            run_transition_hooks(&old, &new, trigger, app, pool).await;
        }
        persist_snapshot(&new, pool).await;
        emit_event(&new, trigger, app);
        Ok(())
    }

    /// Reset to free state. Runs the Pro→Free downgrade hook when the
    /// in-memory state was Pro — even if SQLite's cloud:plan row has
    /// already been deleted (auth::clear runs DELETE FROM settings WHERE
    /// key LIKE 'cloud:%' which used to break the SQLite-key-based
    /// transition guard).
    pub async fn clear(&self, app: &AppHandle, pool: &SqlitePool) -> Result<(), String> {
        self.apply(ProState::free(), Trigger::Clear, app, pool).await
    }

    /// Optimistic boot restore from on-disk snapshots. No hooks. Called
    /// once during app setup before any cloud_get_status fires.
    pub async fn hydrate_from_snapshot(&self, pool: &SqlitePool) {
        let plan = settings::get_by_key(pool, SETTINGS_KEY_PLAN)
            .await
            .ok()
            .flatten()
            .map(|s| s.value)
            .unwrap_or_else(default_plan);
        let credits = settings::get_by_key(pool, SETTINGS_KEY_CREDITS_SNAPSHOT)
            .await
            .ok()
            .flatten()
            .and_then(|s| {
                if s.value.is_empty() {
                    None
                } else {
                    serde_json::from_str::<CloudCredits>(&s.value).ok()
                }
            });
        let subscription = settings::get_by_key(pool, SETTINGS_KEY_SUB_SNAPSHOT)
            .await
            .ok()
            .flatten()
            .and_then(|s| {
                if s.value.is_empty() {
                    None
                } else {
                    serde_json::from_str::<CloudSubscription>(&s.value).ok()
                }
            });
        *self.inner.write() = ProState {
            plan,
            credits,
            subscription,
        };
    }

    /// Build a ProState from a server entitlements DTO and apply. Used by
    /// every Tauri command that returns a `MeResponse` (cloud_get_status,
    /// cloud_exchange_code, link/unlink/update_profile).
    pub async fn apply_from_entitlements(
        &self,
        ent: &CloudEntitlements,
        plan_override: Option<&str>,
        app: &AppHandle,
        pool: &SqlitePool,
    ) -> Result<(), String> {
        let plan = plan_override.unwrap_or(&ent.plan).to_string();
        let new = ProState {
            plan,
            credits: ent.credits.clone(),
            subscription: ent.subscription.clone(),
        };
        self.apply(new, Trigger::Apply, app, pool).await
    }

    /// SSE balance / cloud_ai_balance partial update — patches only
    /// `credits_remaining`, preserves the rest. Goes through `apply` so
    /// the persist hook + event still fire. Preserves the existing split-
    /// write contract where SSE only touches credits, not plan/sub.
    pub async fn patch_credits_remaining(
        &self,
        remaining: i64,
        app: &AppHandle,
        pool: &SqlitePool,
    ) -> Result<(), String> {
        let mut new = self.current();
        if let Some(c) = new.credits.as_mut() {
            c.remaining = remaining;
        } else {
            new.credits = Some(CloudCredits {
                remaining,
                allowance: 0,
                resets_at: None,
            });
        }
        self.apply(new, Trigger::Apply, app, pool).await
    }
}

// ─── Hooks ───────────────────────────────────────────────────────────────────

async fn run_transition_hooks(
    old: &ProState,
    new: &ProState,
    trigger: Trigger,
    app: &AppHandle,
    pool: &SqlitePool,
) {
    let was_pro = old.is_pro();
    let is_pro = new.is_pro();

    // Idempotent cap enforcement. Runs on every non-Pro apply, not just on
    // Pro→Free transitions — closes the bug where leftover-from-old-code
    // coworkers (created while Pro, never disabled when the user signed
    // out under the buggy old transition guard) stayed visibly active on
    // a now-free account. `disable_beyond_first_n` is a no-op when ≤3
    // active rows exist, so this is safe to run on every refresh.
    // Emits `cloud:plan_lapsed` only when rows were actually disabled,
    // so the frontend reload listener doesn't fire on no-op runs.
    if !is_pro {
        let disabled = crate::shared::repos::coworkers::disable_beyond_first_n(pool, 3)
            .await
            .unwrap_or(0);
        if disabled > 0 {
            let _ = app.emit(
                "cloud:plan_lapsed",
                serde_json::json!({
                    "disabled_coworkers": disabled,
                    "trigger": trigger,
                }),
            );
        }
    }

    // Upgrade hook fires on the transition specifically (running enable_all
    // on every Pro refresh would needlessly UPDATE no-op rows). Re-enables
    // any previously-soft-disabled coworkers.
    if !was_pro && is_pro {
        let _ = crate::shared::repos::coworkers::enable_all(pool).await;
        let _ = app.emit(
            "cloud:plan_upgraded",
            serde_json::json!({ "trigger": trigger }),
        );
    }
}

async fn persist_snapshot(state: &ProState, pool: &SqlitePool) {
    // Keep writing the three legacy keys so the existing snapshot-
    // hydration path in +layout.svelte still works without changes.
    // Commit 3 consolidates to a single `cloud:pro_state` blob.
    let _ = settings::upsert(pool, SETTINGS_KEY_PLAN, &state.plan).await;
    if let Some(c) = &state.credits {
        if let Ok(json) = serde_json::to_string(c) {
            let _ = settings::upsert(pool, SETTINGS_KEY_CREDITS_SNAPSHOT, &json).await;
        }
    } else {
        let _ = settings::upsert(pool, SETTINGS_KEY_CREDITS_SNAPSHOT, "").await;
    }
    if let Some(s) = &state.subscription {
        if let Ok(json) = serde_json::to_string(s) {
            let _ = settings::upsert(pool, SETTINGS_KEY_SUB_SNAPSHOT, &json).await;
        }
    } else {
        let _ = settings::upsert(pool, SETTINGS_KEY_SUB_SNAPSHOT, "").await;
    }
}

fn emit_event(state: &ProState, trigger: Trigger, app: &AppHandle) {
    let _ = app.emit(
        "cloud:pro-state",
        serde_json::json!({ "state": state, "trigger": trigger }),
    );
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn pro_state_current(
    manager: tauri::State<'_, ProStateManager>,
) -> ProState {
    manager.current()
}
