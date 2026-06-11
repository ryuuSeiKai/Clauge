# Antigravity Provider — Design Spec

**Date:** 2026-06-09  
**Status:** Approved  
**Scope:** Add `agy` (Antigravity CLI) as a new AI provider in Agent mode.

## Overview

Antigravity CLI (`agy`) is Google's AI coding agent. This spec covers adding it as a
first-class provider in Synapse Agent mode, alongside Claude, Codex, Gemini, and
OpenCode.

The existing `CliRunner` trait → `runner_for()` registry pattern makes adding a new
provider a **1-file Rust impl + registry wiring + frontend enum entry** change.

## Architecture

```
Frontend (Svelte)                       Rust Backend
─────────────────                       ────────────
AgentProvider type      ────────────→   registry::runner_for(id)
  'antigravity'  (NEW)                      │
AGENT_PROVIDERS[]  (NEW entry)              ├─ claude.rs
                                            ├─ codex.rs
                                            ├─ gemini.rs
                                            ├─ opencode.rs
                                            └─ antigravity.rs  (NEW)
                                                 │
                                            CliRunner trait
                                              ├─ binary_name() → "agy"
                                              ├─ build_spawn_command()
                                              ├─ home_dir() → ~/.antigravity
                                              ├─ sessions_root()
                                              ├─ extract_resume_id_from_output()
                                              └─ ...
```

## Changes

### Rust — new file

**`src-tauri/src/shared/cli/antigravity.rs`**

Implement `CliRunner` for AntigravityRunner:

| Trait method | Value / Behaviour |
|---|---|
| `id()` | `"antigravity"` |
| `binary_name()` | `"agy"` |
| `home_dir()` | `~/.antigravity` |
| `plugins_dir()` | `~/.antigravity/extensions` |
| `sessions_root()` | `~/.antigravity/antigravity` |
| `session_file_extension()` | `"jsonl"` |
| `build_spawn_command(opts)` | `agy --prompt-interactive` for new sessions;<br>`agy --conversation <id>` for resume;<br>`--dangerously-skip-permissions` when `opts.skip_permissions` |
| `extract_resume_id_from_output(buf)` | Regex for `agy --conversation <uuid>` |
| `usage_api_*` | `None` (no known public usage API yet) |

### Rust — modified files

**`src-tauri/src/shared/cli/mod.rs`** — add `pub mod antigravity;`

**`src-tauri/src/shared/cli/registry.rs`**:
- Add `use super::antigravity::ANTIGRAVITY;`
- Map `"antigravity"` → `&ANTIGRAVITY` in `runner_for()` and `try_runner_for()`
- Add `"antigravity"` to `SUPPORTED_PROVIDERS`

### Frontend — modified files

**`src/lib/modes/agent/types.ts`**:
- Add `'antigravity'` to `AgentProvider` union type
- Add `{ id: 'antigravity', label: 'Antigravity' }` to `AGENT_PROVIDERS` array

### Optional (follow-up)

- **Not-installed modal** — `AntigravityNotInstalledModal.svelte` if `agy` is not on PATH
- **Usage tracking** — if Antigravity exposes a usage API later
- **Plugin manager support** — `agy plugin` subcommand already exists, can wire up

## Agent Session Flow

1. User creates session with provider = `"antigravity"` + project path
2. Backend builds spawn command: `agy --prompt-interactive --dangerously-skip-permissions`
3. Command runs in PTY terminal inside Agent panel
4. User interacts with `agy` directly (same UX as Claude/Codex/Gemini)
5. On session resume: `agy --conversation <id>`
6. Agent panel shows PTY output, extracts session ID from output markers

## Open Questions (verified at implementation time)

1. Session ID format — UUID or custom? Check `agy` output after first session.
2. System prompt injection — does `agy` have an `--append-system-prompt` equivalent?
3. Session log format — confirm `.jsonl` extension, verify encoder for project-path → dir mapping.

## Risks

- **Low risk** — the pattern is well-established by 4 existing providers
- `agy` CLI flags may change in future versions; binary is user-installed so updates are independent
- No usage analytics endpoint known yet (can add later)
