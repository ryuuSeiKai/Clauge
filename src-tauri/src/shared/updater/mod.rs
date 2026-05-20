//! Channel-aware update flow.
//!
//! The default Tauri updater config has a single static endpoint pointing
//! at the GitHub "latest" release. That works for stable users but means
//! pre-release builds (alpha/rc/beta) get served to everyone — including
//! users who never opted in.
//!
//! This module reads the user's `update_channel` setting (`stable` | `pre`)
//! and resolves the right `latest.json` URL at check time:
//!
//! - `stable` → the GitHub "latest" alias (existing behavior)
//! - `pre`    → the most recent release with `prerelease: true`, looked up
//!              via the GitHub releases API
//!
//! Tauri's Rust-side `updater_builder()` accepts the endpoint per-call, so
//! we plug the resolved URL into a fresh builder each time the frontend
//! triggers an update check.

pub mod channel;
pub mod commands;
pub mod state;
