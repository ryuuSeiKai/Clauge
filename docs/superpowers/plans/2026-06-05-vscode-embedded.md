# VS Code Embedded Tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "Editor" tab that embeds VS Code (`code --serve-web`) as an inline Tauri webview inside the Synapse app.

**Architecture:** A Rust `VscodeServer` manager spawns `code --serve-web` on app boot. The frontend creates an inline child webview in the same window when the Editor mode is active, loading `http://localhost:<port>/`. The mode routing follows the existing stacked-panel pattern in `+page.svelte`.

**Tech Stack:** Rust (Tauri v2 `Webview::addChildWebview`), Svelte 5, VS Code's built-in `--serve-web` mode

---

### File Structure

**New files:**
- `src-tauri/src/modes/editor/mod.rs` — module declarations
- `src-tauri/src/modes/editor/server.rs` — `VscodeServer` struct + lifecycle
- `src/lib/modes/editor/stores.ts` — editor Svelte stores
- `src/lib/modes/editor/commands.ts` — Tauri command wrappers
- `src/lib/modes/editor/components/EditorPanel.svelte` — webview container

**Modified files:**
- `src-tauri/src/modes/mod.rs` — add `pub mod editor`
- `src-tauri/src/lib.rs` — register new module, state, commands
- `src/lib/stores/app.ts` — add `'editor'` to `AppMode` + `VALID_MODES`
- `src/routes/+page.svelte` — import + render `EditorPanel`
- `src/lib/components/sidebar/Sidebar.svelte` — add editor icon button

---

### Task 1: Rust backend — Editor module + VscodeServer

**Files:**
- Create: `src-tauri/src/modes/editor/mod.rs`
- Create: `src-tauri/src/modes/editor/server.rs`
- Modify: `src-tauri/src/modes/mod.rs`

#### Step 1: Create editor module declaration

`src-tauri/src/modes/editor/mod.rs`:
```rust
pub mod server;
pub use server::*;
```

#### Step 2: Create VscodeServer

`src-tauri/src/modes/editor/server.rs`:
```rust
use std::sync::Mutex;
use tauri::Manager;
use tokio::process::Command;

pub struct VscodeServer {
    process: Mutex<Option<tokio::process::Child>>,
    port: u16,
}

impl VscodeServer {
    pub fn new() -> Self {
        Self { process: Mutex::new(None), port: 8420 }
    }

    pub async fn start(&self, project_path: &str) -> Result<u16, String> {
        let mut port = 8420u16;
        let mut child = None;
        for attempt in 0..5 {
            let test_port = port + attempt;
            let check = std::process::Command::new("sh")
                .args(["-c", &format!("lsof -i :{} -P 2>/dev/null | grep -q LISTEN && echo in-use || echo free", test_port)])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "in-use")
                .unwrap_or(false);
            if check { continue; }

            // Check if `code` command exists
            let which = std::process::Command::new("which").arg("code").output();
            if which.map(|o| !o.status.success()).unwrap_or(true) {
                return Err("VS Code not found. Install VS Code and add `code` to PATH.".to_string());
            }

            let cmd = Command::new("code")
                .args([
                    "--serve-web",
                    "--port", &test_port.to_string(),
                    "--without-connection-token",
                    "--accept-server-license-terms",
                    project_path,
                ])
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| format!("Failed to start VS Code server: {}", e))?;

            child = Some(cmd);
            port = test_port;
            break;
        }
        let child = child.ok_or_else(|| "All ports 8420-8424 are in use".to_string())?;
        *self.process.lock().unwrap() = Some(child);
        self.port = port;
        Ok(port)
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.process.lock().unwrap().take() {
            let _ = child.start_kill();
        }
    }

    pub fn port(&self) -> u16 { self.port }
}

impl Drop for VscodeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tauri::command]
pub fn editor_get_port(state: tauri::State<'_, VscodeServer>) -> Result<u16, String> {
    Ok(state.port())
}

#[tauri::command]
pub async fn editor_open_project(
    state: tauri::State<'_, VscodeServer>,
    project_path: String,
) -> Result<u16, String> {
    state.stop();
    // Brief delay so the port releases
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    state.start(&project_path).await
}
```

#### Step 3: Register in modes/mod.rs

Add after existing mod lines:
```rust
pub mod editor;
```

---

### Task 2: Register VscodeServer in lib.rs

**File:** Modify `src-tauri/src/lib.rs`

#### Step 1: Find the app setup and add state + commands

In the `.setup()` closure, after existing state registrations:
```rust
use crate::modes::editor::VscodeServer;

// After existing managed state registrations:
app.manage(VscodeServer::new());
```

In the `.invoke_handler(tauri::commands::generate_handler![` array, add:
```rust
modes::editor::editor_get_port,
modes::editor::editor_open_project,
```

Also start the VS Code server on boot (inside setup after manage):
```rust
// Start VS Code server, best-effort (user may not have `code` installed)
let editor_server = app.state::<VscodeServer>();
let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
tauri::async_runtime::spawn(async move {
    match editor_server.start(&home).await {
        Ok(port) => log::info!("VS Code server started on port {}", port),
        Err(e) => log::warn!("VS Code server: {}", e),
    }
});
```

---

### Task 3: Frontend — AppMode + Editor stores + commands

**Files:**
- Modify: `src/lib/stores/app.ts`
- Create: `src/lib/modes/editor/stores.ts`
- Create: `src/lib/modes/editor/commands.ts`

#### Step 1: Add 'editor' to AppMode

In `src/lib/stores/app.ts`:
```typescript
export type AppMode = 'agent' | 'rest' | 'sql' | 'nosql' | 'ssh' | 'explorer' | 'workspace' | 'history' | 'editor';

const VALID_MODES: AppMode[] = ['agent', 'rest', 'sql', 'nosql', 'ssh', 'explorer', 'workspace', 'editor'];
```

#### Step 2: Create editor stores

`src/lib/modes/editor/stores.ts`:
```typescript
import { writable } from 'svelte/store';

export const editorPort = writable<number | null>(null);
export const editorProjectPath = writable<string>('');
```

#### Step 3: Create editor commands

`src/lib/modes/editor/commands.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core';

export const editorGetPort = () => invoke<number>('editor_get_port');
export const editorOpenProject = (projectPath: string) => invoke<number>('editor_open_project', { projectPath });
```

---

### Task 4: Frontend — EditorPanel with inline webview

**File:**
- Create: `src/lib/modes/editor/components/EditorPanel.svelte`
- NOTE: This must be created inside the `editor/components/` directory structure. If you need to create intermediate directories, do so.

#### Step 1: Create the panel component

`src/lib/modes/editor/components/EditorPanel.svelte`:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Webview } from '@tauri-apps/api/webview';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { editorPort, editorProjectPath } from '../stores';
  import { editorGetPort } from '../commands';
  import { get } from 'svelte/store';

  let containerEl: HTMLDivElement;
  let webview: Webview | null = null;
  let loading = $state(true);
  let error = $state<string | null>(null);
  let port = $state<number | null>(null);

  onMount(async () => {
    try {
      port = await editorGetPort();
      editorPort.set(port);
    } catch (e) {
      error = String(e);
      loading = false;
      return;
    }

    if (!port) { error = 'VS Code server not available'; loading = false; return; }

    // Wait for VS Code server to be ready (poll /healthz or just wait)
    await waitForServer(port);

    loading = false;

    try {
      const win = getCurrentWindow();
      const rect = containerEl.getBoundingClientRect();

      // @ts-ignore — addChildWebview is available in Tauri v2 but TS types may lag
      webview = await win.addChildWebview({
        url: `http://localhost:${port}/`,
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      });

      // ResizeObserver to keep webview bounds in sync
      const ro = new ResizeObserver((entries) => {
        for (const entry of entries) {
          const r = entry.contentRect;
          webview?.setBounds({
            x: Math.round(r.x),
            y: Math.round(r.y),
            width: Math.round(r.width),
            height: Math.round(r.height),
          }).catch(() => {});
        }
      });
      ro.observe(containerEl);

      // Cleanup observer on destroy
      onDestroy(() => {
        ro.disconnect();
        webview?.close().catch(() => {});
        webview = null;
      });
    } catch (e) {
      error = `Failed to create editor: ${e}`;
    }
  });

  async function waitForServer(port: number, maxAttempts = 30) {
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const resp = await fetch(`http://localhost:${port}/`, { method: 'HEAD' });
        if (resp.ok || resp.status === 200) return;
      } catch { /* server not ready yet */ }
      await new Promise(r => setTimeout(r, 1000));
    }
  }
</script>

<div class="editor-root">
  {#if loading}
    <div class="editor-status">
      <svg class="editor-spinner" viewBox="0 0 24 24" width="24" height="24">
        <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <span>Starting VS Code...</span>
    </div>
  {:else if error}
    <div class="editor-error">
      <svg viewBox="0 0 24 24" width="24" height="24"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
      <p>{error}</p>
      <button class="editor-retry-btn" onclick={() => { loading = true; error = null; onMount(); }}>Retry</button>
    </div>
  {/if}
  <div bind:this={containerEl} class="editor-container" class:active={!loading && !error}></div>
</div>

<style>
  .editor-root {
    flex: 1;
    display: flex;
    position: relative;
    overflow: hidden;
  }
  .editor-container {
    position: absolute;
    inset: 0;
    visibility: hidden;
  }
  .editor-container.active {
    visibility: visible;
  }
  .editor-status {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--t3);
    font-size: 13px;
    font-family: var(--ui);
  }
  .editor-spinner {
    animation: spin 1s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .editor-error {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--err);
    font-size: 13px;
    font-family: var(--ui);
    padding: 24px;
  }
  .editor-error p { color: var(--t2); text-align: center; }
  .editor-retry-btn {
    padding: 6px 16px;
    border-radius: 6px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t1);
    font-size: 12px;
    cursor: default;
  }
  .editor-retry-btn:hover { background: var(--surface-hover); }
</style>
```

---

### Task 5: Frontend — Register EditorPanel in +page.svelte

**File:** Modify `src/routes/+page.svelte`

#### Step 1: Add import
After the workspace panel import, add:
```typescript
import EditorPanel from '$lib/modes/editor/components/EditorPanel.svelte';
```

#### Step 2: Add panel div
After the Workspace panel div (before Settings overlay panel comment), add:
```svelte
<div class="panel" class:active={$mode === 'editor' && !settingsActive}>
  <EditorPanel />
</div>
```

---

### Task 6: Frontend — Add sidebar button

**File:** Modify `src/lib/components/sidebar/Sidebar.svelte`

#### Step 1: Add editor icon button

After the Explorer sidebar button and before the separator (`<div class="sb-sep">`), add:
```svelte
<SidebarButton label="Editor" tip="Editor (VS Code)" active={$mode === 'editor'} id="sbi-editor" onclick={() => setMode('editor')}>
  <!-- Code/angle brackets — "code editor" -->
  <svg viewBox="0 0 24 24"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
</SidebarButton>
```

---

### Task 7: Keyboard shortcut Cmd+Shift+E

**File:** Modify `src/routes/+layout.svelte`

#### Step 1: Add keyboard handler

Find the existing keyboard handlers section (around line 1150 where `window.addEventListener("focus", ...)` is). Add after the existing `loadAgentClaudePlan()` call:

```typescript
// Editor tab keyboard shortcut: Cmd+Shift+E (macOS) / Ctrl+Shift+E (Windows/Linux)
window.addEventListener("keydown", (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'e') {
    e.preventDefault();
    const { mode } = require('$lib/stores/app');
    mode.set('editor');
  }
});
```

Note: `require()` won't work in ESM. Use dynamic import or reference the already-imported `mode` store. The `mode` store should already be imported in `+layout.svelte`. Search for `import { mode, ... } from '$lib/stores/app'` at the top of the script section and use that reference instead:

```typescript
// Add this inside the onMount where other listeners are registered:
window.addEventListener("keydown", (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'e') {
    e.preventDefault();
    import('$lib/stores/app').then(({ mode }) => mode.set('editor'));
  }
});
```

### Task 8: Test and verify

1. Run `bun tauri dev`
2. Click the new Editor icon in the sidebar
3. Verify: loading state shows, then VS Code appears as a webview
4. Click another mode → editor hides
5. Click Editor again → editor shows (server keeps running)
6. Close app → VS Code process is killed
7. If `code` not in PATH → error message with Retry button
