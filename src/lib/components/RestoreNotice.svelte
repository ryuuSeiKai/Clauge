<script lang="ts">
  // Top-of-app banner shown briefly after a cloud restore when at least
  // one connection across SSH / SQL / NoSQL / Explorer came back without
  // its credentials. Reminds the developer that secrets are per-device
  // by design; auto-hides via the store's timer so it doesn't linger.

  import { fly } from 'svelte/transition';
  import { restoreNotice, dismissRestoreNotice, type RestoreMode } from '$lib/stores/missingCredentials';

  // Slide in from the right to match Toast's entrance.
  const SLIDE_PX = 20;

  const LABEL: Record<RestoreMode, string> = {
    ssh: 'SSH',
    sql: 'SQL',
    nosql: 'NoSQL',
    explorer: 'Explorer',
  };

  function joinModes(modes: RestoreMode[]): string {
    const labels = modes.map((m) => LABEL[m]);
    if (labels.length <= 1) return labels[0] ?? '';
    if (labels.length === 2) return `${labels[0]} and ${labels[1]}`;
    return `${labels.slice(0, -1).join(', ')}, and ${labels[labels.length - 1]}`;
  }
</script>

{#if $restoreNotice.modes.length > 0}
  <div class="rn-wrap" transition:fly={{ x: SLIDE_PX, duration: 200 }}>
    <div class="rn-card" role="status" aria-live="polite">
      <svg class="rn-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 12a9 9 0 11-3-6.7" />
        <polyline points="21 3 21 9 15 9" />
      </svg>
      <span class="rn-text">
        Restored from cloud. Passwords and keys stay per-device, so re-enter them in {joinModes($restoreNotice.modes)} when you reconnect.
      </span>
      <button class="rn-close" onclick={dismissRestoreNotice} aria-label="Dismiss">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
    </div>
  </div>
{/if}

<style>
  /* Anchored to the same bottom-right corner as Toast, sitting just
   * above the standard toast stack (matched bottom offset + a small
   * gap so they don't overlap when both appear at once). Wider than a
   * toast since the message is longer; wraps cleanly across multiple
   * lines. */
  .rn-wrap {
    position: fixed;
    bottom: 40px;
    right: 20px;
    z-index: 1201;
    pointer-events: none;
  }
  .rn-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 12px 11px 14px;
    background: var(--n);
    border: 1px solid var(--b1);
    border-radius: var(--radius-lg);
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.32);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    color: var(--t1);
    font-family: var(--ui);
    font-size: 12px;
    line-height: 1.5;
    max-width: 380px;
    pointer-events: auto;
  }
  .rn-icon { flex-shrink: 0; color: var(--acc); margin-top: 1px; }
  .rn-text {
    color: var(--t2);
    word-break: break-word;
    overflow-wrap: anywhere;
  }
  .rn-close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    margin-left: 2px;
    flex-shrink: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--t4);
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .rn-close:hover { background: var(--c); color: var(--t1); }
</style>
