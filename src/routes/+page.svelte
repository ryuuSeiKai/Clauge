<script lang="ts">
  import { mode, lastModeBeforeEditor } from '$lib/stores/app';
  import { activeHistoryEntry } from '$lib/modes/rest/stores';
  import { tabs as sharedTabs, activeTabId } from '$lib/shared/stores/tabs';
  import AgentPanel from '$lib/modes/agent/components/AgentPanel.svelte';
  import RestPanel from '$lib/modes/rest/components/RestPanel.svelte';
  import SqlPanel from '$lib/modes/sql/components/SqlPanel.svelte';
  import NoSqlPanel from '$lib/modes/nosql/components/NoSqlPanel.svelte';
  import SshPanel from '$lib/modes/ssh/components/SshPanel.svelte';
  import ExplorerPanel from '$lib/modes/explorer/components/ExplorerPanel.svelte';
  import WorkspacePanel from '$lib/modes/workspace/components/WorkspacePanel.svelte';
  import HistoryViewer from '$lib/modes/rest/components/HistoryViewer.svelte';
  import EditorPanel from '$lib/modes/editor/components/EditorPanel.svelte';
  import SettingsModal from '$lib/components/settings/SettingsModal.svelte';

  const effectiveMode = $derived($mode === 'editor' ? $lastModeBeforeEditor : $mode);

  // Settings is the only cross-mode "tab" — visibility is driven by the
  // active topbar tab, not $mode (which stays tied to the user's real
  // mode so the "+" button + AI panel stay correct).
  const settingsActive = $derived(
    !!$sharedTabs.find((t) => t.id === $activeTabId && t.mode === 'settings'),
  );
</script>

<!--
  All mode panels are mounted continuously; only the active one is visible.
  This preserves expensive per-mode state (xterm terminals + SSH Handles in
  Agent / SSH, SFTP sessions in Explorer, CodeMirror editors + result tables
  in SQL/NoSQL, scroll/focus state everywhere) across mode switches.

  Previously this used `{#if mode === 'X'}` per panel, which unmounted the
  panel on every mode switch and triggered each panel's `onDestroy` — that
  killed terminal PTYs, SSH `Handle`s, and SFTP sessions. Switching back
  reconnected from scratch (re-auth, OTP prompt, etc.).

  Stacking with `position: absolute; inset: 0` + visibility/pointer-events
  toggle keeps each panel sized correctly even while hidden (visibility:
  hidden keeps layout). Performance cost: idle panels hold a Svelte
  component but no active resources (terminals only spawn when the user
  opens a session inside that mode).
-->
<!--
  When Settings is the active tab, ALL mode panels are forced inactive
  (visibility: hidden). The mode store ($mode) intentionally stays at
  the user's real mode so the "+" button / AI panel / sidebar selection
  stay correct — only the panel's `active` class is gated. In solid
  themes this didn't matter (opaque mode panel + opaque Settings panel
  on top = Settings covers everything). In glass mode both panels are
  rgba, so the underlying mode panel was bleeding through Settings,
  making text + icons illegible.
-->
<div class="workspace" class:has-editor={$mode === 'editor'}>
  <div class="panel" class:active={effectiveMode === 'agent' && !settingsActive}>
    <AgentPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'history' && !settingsActive}>
    {#if $activeHistoryEntry}
      <HistoryViewer />
    {:else}
      <div class="history-empty">
        <svg viewBox="0 0 24 24" width="36" height="36"><circle cx="12" cy="12" r="10" stroke="var(--t4)" fill="none" stroke-width="1.2"/><path d="M12 6v6l4 2" stroke="var(--t4)" fill="none" stroke-width="1.2" stroke-linecap="round"/></svg>
        <span>Select an entry from history to view details</span>
      </div>
    {/if}
  </div>

  <div class="panel" class:active={effectiveMode === 'rest' && !settingsActive}>
    <RestPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'sql' && !settingsActive}>
    <SqlPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'nosql' && !settingsActive}>
    <NoSqlPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'ssh' && !settingsActive}>
    <SshPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'explorer' && !settingsActive}>
    <ExplorerPanel />
  </div>

  <div class="panel" class:active={effectiveMode === 'workspace' && !settingsActive}>
    <WorkspacePanel />
  </div>

  <div class="panel editor-panel" class:active={$mode === 'editor' && !settingsActive}>
    <EditorPanel />
  </div>

  <!-- Settings overlay panel — sits above all mode panels when its tab
       is the active topbar tab (z-index: 2 in the .active rule below). -->
  <div class="panel panel-settings" class:active={settingsActive}>
    <SettingsModal />
  </div>
</div>

<style>
  .workspace {
    flex: 1;
    /* Becomes the containing block for the absolutely-positioned panels. */
    position: relative;
    min-height: 0;
    overflow: hidden;
  }
  .panel {
    /* Stack — all panels share the same rectangle, fill the workspace. */
    position: absolute;
    inset: 0;
    display: flex;
    /* Hidden by default. `visibility: hidden` (rather than display:none)
       keeps each panel's layout calculated, so xterm.js/CodeMirror don't
       see a 0×0 container and miscalibrate when the panel becomes active. */
    visibility: hidden;
    pointer-events: none;
  }
  .panel.active {
    visibility: visible;
    pointer-events: auto;
    /* Float above siblings — needed because all panels share the same
       z-index plane otherwise. */
    z-index: 1;
  }
  /* Settings overlays whatever mode panel is also marked .active, so it
     wins the stacking order while its tab is focused. Sidebar mode-click
     handlers (realignActiveTabToMode) move focus off the settings tab,
     so this only fires when settings is genuinely the active tab. */
  .panel.panel-settings.active {
    z-index: 2;
  }
  /* Editor split — when active, .workspace becomes flex row */
  .workspace.has-editor {
    display: flex;
    flex-direction: row;
  }

  .workspace.has-editor .panel.active:not(.editor-panel) {
    position: relative;
    flex: 1;
  }

  .workspace.has-editor .panel.editor-panel {
    position: relative;
    width: min(50%, 800px);
    min-width: 400px;
    visibility: visible;
    pointer-events: auto;
    border-left: 1px solid #30363d;
  }
  .history-empty {
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
