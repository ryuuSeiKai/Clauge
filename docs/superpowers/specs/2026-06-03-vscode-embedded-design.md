# VS Code Embedded Tab — Design Spec

**Date:** 2026-06-03
**Project:** Synapse (formerly Synape)
**Status:** Approved design

## Overview

Embed VS Code (`code --serve-web`) as a native tab inside the Synapse Tauri app, allowing users to edit code directly without leaving the application. Uses the existing VS Code installation on the user's machine.

## Architecture

### Components

```
┌──────────────────────────────────────────────┐
│  Synapse Window                               │
│  ┌─────────┐ ┌────────────────────────────┐  │
│  │ Sidebar │ │  Content Area               │  │
│  │         │ │  ┌──────────────────────┐   │  │
│  │  Agent  │ │  │  Tauri Webview       │   │  │
│  │  REST   │ │  │  (localhost:8420)    │   │  │
│  │  SQL    │ │  │  ┌────────────────┐  │   │  │
│  │  ...    │ │  │  │ VS Code Web UI │  │   │  │
│  │  EDITOR │ │  │  │                │  │   │  │
│  │         │ │  │  │                │  │   │  │
│  └─────────┘ │  └──┴────────────────┴──┘   │  │
│              └────────────────────────────┘  │
└──────────────────────────────────────────────┘
```

### Rust Backend — VS Code Server Manager

**File:** `src-tauri/src/modes/editor/server.rs`

**VscodeServer struct:**
- `process: Option<tokio::process::Child>` — handle to the spawned `code --serve-web` process
- `port: u16` — the port on which the VS Code web server is listening

**Lifecycle:**
1. **Boot:** During app setup, `VscodeServer::start()` spawns `code --serve-web --port <port> --without-connection-token --accept-server-license-terms`.
   - Port: starts at 8420, retries +1 if busy
   - Project path: determined by the active agent session's `worktreePath` or `projectPath`, or falls back to `$HOME`
   - `--without-connection-token` disables the one-time auth token so the webview can connect directly
2. **Tab switch:** The server stays running. Tab show/hide controls webview visibility.
3. **App exit:** `Drop` impl kills the process (SIGTERM, fallback SIGKILL).

**Tauri Commands:**
- `editor_get_port` → returns the VS Code server port (for frontend webview URL)
- `editor_get_project_path` → returns current project path
- `editor_open_project(path)` → stops current VS Code server, starts new one at given path

**Registration:**
- `VscodeServer` as Tauri managed state (`tauri::State<VscodeServer>`)
- Commands registered in `src-tauri/src/lib.rs`'s `.invoke_handler()`

### Frontend — Editor Mode

**App Mode:**
- Add `'editor'` to `AppMode` union type in `src/lib/stores/app.ts`

**Sidebar:**
- Add editor icon (code bracket icon) between SSH and Workspace
- Click → sets mode to `'editor'`

**EditorPanel.svelte** (new, `src/lib/modes/editor/components/EditorPanel.svelte`):
- On mount: create a child webview in the current window using Tauri's `Webview.addChildWebview()`
- URL: `http://localhost:{port}/`
- Position: fills the content area (right of sidebar)
- ResizeObserver: syncs webview bounds to container size
- On tab hide: `webview.hide()`
- On tab show: `webview.show()`
- On destroy: remove the child webview (server stays running)

**Project path integration:**
- When an Agent session is active, the editor opens at `session.worktreePath || session.projectPath`
- When no session is active, the editor opens at the last-used path or `$HOME`
- The Agent's current working directory is synced to the editor via `editor_open_project`

### State Management

```typescript
// src/lib/stores/editor.ts
export const editorPort = writable<number | null>(null);
export const editorProjectPath = writable<string>('');
export const editorWebview = writable<Webview | null>(null);
```

- `editorPort` — set by `editor_get_port` on boot
- `editorProjectPath` — syncs with active agent session path
- `editorWebview` — reference to the Tauri child webview for show/hide/resize

### Tab Registration

Register the editor tab in `+layout.svelte` alongside other modes:
- `mode === 'editor'` → render `EditorPanel`
- Sidebar button uses `mode.set('editor')`

### Keyboard shortcuts

- `Cmd+Shift+E` → switch to Editor tab (consistent with VS Code's own shortcut)

## Error Handling

| Scenario | Behavior |
|---|---|
| `code` command not found | Show toast: "VS Code not found. Install VS Code and add `code` to PATH." |
| VS Code server fails to start | Retry on next port up to 5 attempts, then show error toast |
| VS Code server crashes | Auto-restart with a small delay, show "Reconnecting..." indicator |
| Port already in use | Try +1 incrementally up to 5 ports, then fail |
| Webview fails to load | Show inline error with "Retry" button |

## Files Changed / Created

### New files:
- `src-tauri/src/modes/editor/mod.rs` — module declaration
- `src-tauri/src/modes/editor/server.rs` — VscodeServer struct + lifecycle
- `src/lib/modes/editor/stores.ts` — editor stores
- `src/lib/modes/editor/commands.ts` — editor Tauri command wrappers
- `src/lib/modes/editor/components/EditorPanel.svelte` — webview container

### Modified files:
- `src-tauri/src/lib.rs` — register editor module + commands + state
- `src-tauri/src/modes/mod.rs` — add `pub mod editor`
- `src/lib/stores/app.ts` — add `'editor'` to `AppMode`
- `src/routes/+layout.svelte` — add sidebar icon + tab router for editor mode
- `src/lib/shared/constants/mod.ts` or similar — add `Cmd+Shift+E` shortcut

## Out of Scope (v1)

- Multiple VS Code instances for different projects
- Sync editor tabs with agent session switching
- File tree sidebar inside Synapse (VS Code has its own)
- Terminal integration (VS Code has integrated terminal)
- Extensions management
