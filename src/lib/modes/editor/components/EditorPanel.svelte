<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Webview } from '@tauri-apps/api/webview';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
  import { editorPort, editorProjectPath } from '../stores';
  import { editorOpenProject, editorSyncTheme } from '../commands';
  import { activeAgentSession, agentSessions } from '$lib/modes/agent/stores';
  import { mode } from '$lib/stores/app';
  import { settings, setSetting } from '$lib/stores/settings';
  import { get } from 'svelte/store';

  let containerEl: HTMLDivElement;
  // When true, open the worktree copy (isolated branch). Default is
  // to open the original project with its local branches.
  let useWorktree = $derived(($settings["editor.useWorktree"] ?? "false") === "true");

  // Deduplicated project list from agent sessions for the project picker.
  let projects = $derived(
    [...new Map(
      $agentSessions
        .filter(s => s.projectPath)
        .map(s => [s.projectPath, { path: s.projectPath, name: s.projectName, provider: s.provider }])
    ).values()]
  );
  let showProjects = $state(false);
  document.addEventListener('click', onDocClick);
  let webviewObj: Webview | null = $state(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let port = $state<number | null>(null);
  let ro: ResizeObserver | null = null;
  let currentPath = $state('');
  let initialized = $state(false);
  // Non-empty when the user picked a project from the picker — overrides
  // the session-derived path for this init cycle.
  let pendingPath = $state('');

  async function initEditor() {
    try {
      let targetPath = pendingPath;
      pendingPath = '';
      if (!targetPath) {
        const session = get(activeAgentSession);
        targetPath = useWorktree
          ? (session?.worktreePath || session?.projectPath || '')
          : (session?.projectPath || '');
      }

      // Always start a fresh server — editorGetPort doesn't spawn one.
      port = await editorOpenProject(targetPath || '');
      editorPort.set(port);
      editorProjectPath.set(targetPath);
      currentPath = targetPath;
    } catch (e) {
      error = String(e);
      loading = false;
      return;
    }

    if (!port) { error = 'VS Code server not available'; loading = false; return; }

    // Sync Synapse theme colors to VS Code Machine settings
    syncThemeColors();

    await waitForServer(port);
    loading = false;
    // Wait for Svelte to update the DOM AND for CSS layout to complete.
    // tick() alone isn't enough — layout may not be calculated yet.
    // Double rAF ensures at least one layout+paint cycle has finished.
    await tick();
    await new Promise<void>(r => requestAnimationFrame(() => requestAnimationFrame(() => r())));

    try {
      const win = getCurrentWindow();
      // Use the .workspace element directly — it defines the exact content
      // area the webview should fill. containerEl may not have settled
      // its layout yet even after double-rAF if the editor-root chain
      // hasn't been fully laid out.
      const workspace = document.querySelector('.workspace');
      const rect = workspace?.getBoundingClientRect() ?? containerEl.getBoundingClientRect();
      const targetEl = workspace ?? containerEl;

      // Reserve ~30 px at the top for the HTML toolbar so the native
      // webview doesn't cover the close button / project picker.
      const TOOLBAR_H = 30;

      const label = `vscode-editor-${port}`;
      webviewObj = new Webview(win, label, {
        url: `http://127.0.0.1:${port}/`,
        x: Math.round(rect.x),
        y: Math.round(rect.y) + TOOLBAR_H,
        width: Math.round(rect.width) || 800,
        height: Math.max(100, Math.round(rect.height) - TOOLBAR_H),
        focus: true,
        transparent: true,
        backgroundColor: [0, 0, 0, 0],
      });

      await new Promise<void>((resolve, reject) => {
        webviewObj!.once('tauri://created', () => resolve());
        webviewObj!.once('tauri://error', (e: any) => reject(new Error(String(e?.payload ?? e))));
        setTimeout(() => reject(new Error('Webview creation timed out')), 8000);
      });

      // Explicitly bring the webview to front — on some platforms it may
      // start behind the main window.
      webviewObj!.show().catch(() => {});
      webviewObj!.setFocus().catch(() => {});
      // Let the webview auto-resize to fill the container, then keep it
      // in sync with ResizeObserver as a fallback.
      webviewObj!.setAutoResize(true).catch(() => {});

      const observed = targetEl as Element;
      ro = new ResizeObserver((_entries) => {
        const r = observed.getBoundingClientRect();
        webviewObj?.setPosition(new LogicalPosition(Math.round(r.x), Math.round(r.y) + TOOLBAR_H)).catch(() => {});
        webviewObj?.setSize(new LogicalSize(Math.round(r.width), Math.max(100, Math.round(r.height) - TOOLBAR_H))).catch(() => {});
      });
      ro.observe(observed);
      initialized = true;
    } catch (e) {
      error = `Failed to create editor: ${e}`;
    }
  }

  let initRequested = false;

  // Only init when the user switches to the Editor tab — don't consume
  // resources (VS Code server, webview) until needed.
  $effect(() => {
    if ($mode === 'editor') {
      const session = get(activeAgentSession);
      const expected = useWorktree
        ? (session?.worktreePath || session?.projectPath || '')
        : (session?.projectPath || '');
      if (!initialized && !initRequested) {
        initRequested = true;
        loading = true;
        initEditor();
      } else if (initialized && expected && expected !== currentPath) {
        // Path changed (e.g. user toggled worktree) — restart server.
        currentPath = expected;
        editorProjectPath.set(expected);
        webviewObj?.close().catch(() => {});
        webviewObj = null;
        ro?.disconnect();
        ro = null;
        loading = true;
        error = null;
        initEditor();
      }
    }
  });



  // Keep the native webview alive when leaving the Editor tab — hide it
  // and show it again when coming back, to preserve editor state.
  let wasEditor = $state(false);
  $effect(() => {
    if ($mode === 'editor') {
      if (!wasEditor && initialized) {
        webviewObj?.show().catch(() => {});
        webviewObj?.setFocus().catch(() => {});
      }
      wasEditor = true;
    } else {
      if (wasEditor) {
        wasEditor = false;
        webviewObj?.hide().catch(() => {});
      }
    }
  });

  // Watch for agent session changes: reopen the editor at the new path.
  $effect(() => {
    if (!initialized) return;
    const session = $activeAgentSession;
    const newPath = useWorktree
      ? (session?.worktreePath || session?.projectPath || '')
      : (session?.projectPath || '');
    if (newPath && newPath !== currentPath) {
      currentPath = newPath;
      editorProjectPath.set(newPath);
      webviewObj?.close().catch(() => {});
      webviewObj = null;
      ro?.disconnect();
      ro = null;
      loading = true;
      error = null;
      initEditor();
    }
  });

  function onDocClick(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (!t.closest('.editor-projects-wrap')) showProjects = false;
  }

  onDestroy(() => {
    document.removeEventListener('click', onDocClick);
    ro?.disconnect();
    webviewObj?.close().catch(() => {});
    webviewObj = null;
  });

  function reopenProject(projectPath: string) {
    showProjects = false;
    webviewObj?.close().catch(() => {});
    webviewObj = null;
    ro?.disconnect();
    ro = null;
    loading = true;
    error = null;
    pendingPath = projectPath;
    initEditor();
  }

  function syncThemeColors() {
    const style = getComputedStyle(document.documentElement);
    const theme = {
      accent: style.getPropertyValue('--acc').trim() || '#6b8cff',
      foreground: style.getPropertyValue('--t1').trim() || '#cccccc',
      border: style.getPropertyValue('--b1').trim() || '#ffffff20',
      selection: (style.getPropertyValue('--acc').trim() || '#6b8cff') + '20',
    };
    editorSyncTheme(JSON.stringify(theme)).catch(() => {});
  }

  async function waitForServer(port: number) {
    // First run downloads the VS Code server (~50 MB), which can take
    // a while. 8 s covers most cases; the webview itself will show a
    // "downloading" page and auto-reload once ready.
    await new Promise(r => setTimeout(r, 8000));
  }
</script>

<div class="editor-root">
  {#if initialized && !loading && !error}
    <div class="editor-toolbar">
      <label class="editor-wt-toggle" title="Open the worktree copy (isolated branch). Default: original project with local branches.">
        <input
          type="checkbox"
          checked={useWorktree}
          onchange={(e) => setSetting("editor.useWorktree", String(e.currentTarget.checked))}
        />
        <span>Worktree</span>
      </label>
      {#if projects.length > 0}
        <div class="editor-projects-wrap">
          <button class="editor-projects-btn" onclick={() => showProjects = !showProjects}>
            <svg viewBox="0 0 24 24" width="14" height="14"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
            <span>{currentPath ? (projects.find(p => p.path === currentPath)?.name || '···') : 'Projects'}</span>
            <svg viewBox="0 0 24 24" width="10" height="10"><polyline points="6 9 12 15 18 9"/></svg>
          </button>
          {#if showProjects}
            <div class="editor-projects-drop" role="listbox">
              {#each projects as p}
                <button
                  class="editor-projects-item"
                  class:active={p.path === currentPath}
                  onclick={() => reopenProject(p.path)}
                >
                  <span class="epi-name">{p.name}</span>
                  <span class="epi-path">{p.path}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      <div class="editor-toolbar-spacer"></div>
      <button class="editor-close-btn" onclick={() => mode.set('agent')} title="Close editor (Cmd+Shift+E)">
        <svg viewBox="0 0 24 24" width="16" height="16"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
      </button>
    </div>
  {/if}
  {#if !initialized && !loading}
    <div class="editor-placeholder">
      <svg viewBox="0 0 24 24" width="36" height="36"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
      <span>Open the Editor tab to launch VS Code</span>
    </div>
  {:else if loading}
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
      <button class="editor-retry-btn" onclick={() => { loading = true; error = null; initEditor(); }}>Retry</button>
    </div>
  {/if}
  <div bind:this={containerEl} class="editor-container" class:active={!loading && !error && initialized}></div>
</div>

<style>
  .editor-root {
    flex: 1;
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
  }
  .editor-toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 4px 12px;
    border-bottom: 1px solid var(--b1);
    background: var(--surface);
  }
  .editor-wt-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--t2);
    font-family: var(--ui);
    cursor: default;
    user-select: none;
  }
  .editor-wt-toggle input { cursor: default; }
  .editor-toolbar-spacer { flex: 1; }
  .editor-close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--t2);
    cursor: default;
    border-radius: 4px;
    padding: 0;
  }
  .editor-close-btn:hover { background: var(--surface-hover); color: var(--t1); }
  .editor-projects-wrap { position: relative; margin-left: 8px; }
  .editor-projects-btn {
    display: flex; align-items: center; gap: 6px;
    padding: 2px 8px; height: 22px;
    border: 1px solid var(--b1); border-radius: 4px;
    background: transparent; color: var(--t2);
    font-size: 11px; font-family: var(--ui); cursor: default;
  }
  .editor-projects-btn:hover { background: var(--surface-hover); }
  .editor-projects-drop {
    position: absolute; top: 100%; left: 0; margin-top: 4px;
    min-width: 260px; max-height: 240px; overflow-y: auto;
    background: var(--surface); border: 1px solid var(--b1);
    border-radius: 6px; box-shadow: 0 4px 16px rgba(0,0,0,0.3);
    z-index: 10; padding: 4px;
  }
  .editor-projects-item {
    display: flex; flex-direction: column; gap: 1px;
    width: 100%; padding: 6px 10px; border: none; border-radius: 4px;
    background: transparent; cursor: default;
    text-align: left; font-family: var(--ui);
  }
  .editor-projects-item:hover { background: var(--surface-hover); }
  .editor-projects-item.active { background: var(--surface-hover); }
  .epi-name { font-size: 12px; color: var(--t1); }
  .epi-path { font-size: 10px; color: var(--t3); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
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
  .editor-placeholder {
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
</style>
