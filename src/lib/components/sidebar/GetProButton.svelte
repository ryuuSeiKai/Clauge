<script lang="ts">
  import { cloudPlan, upgradeModalOpen } from '$lib/stores/cloud';

  const isPro = $derived($cloudPlan === 'pro');
</script>

{#if isPro}
  <span class="pill pill-pro" title="Clauge Pro active">
    <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 2l2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6L12 2z"/>
    </svg>
    PRO
  </span>
{:else}
  <button
    class="pill pill-free"
    onclick={(e) => { e.stopPropagation(); upgradeModalOpen.set(true); }}
    title="Upgrade to Clauge Pro"
  >
    <span class="lbl-free">FREE</span>
    <span class="sep">·</span>
    <span class="lbl-cta">Get Pro</span>
    <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M5 12h14"/><path d="M12 5l7 7-7 7"/>
    </svg>
  </button>
{/if}

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 9px;
    font-family: var(--ui);
    font-weight: 700;
    line-height: 1.4;
    letter-spacing: 0.05em;
    white-space: nowrap;
    background: color-mix(in srgb, var(--acc, #c2185b) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--acc, #c2185b) 45%, transparent);
    color: var(--acc, #c2185b);
  }
  .pill-free {
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
  }
  .pill-free:hover {
    background: color-mix(in srgb, var(--acc, #c2185b) 25%, transparent);
    border-color: var(--acc, #c2185b);
  }
  .pill-pro {
    background: var(--acc, #c2185b);
    border-color: var(--acc, #c2185b);
    color: white;
  }
  .lbl-free {
    opacity: 0.7;
    font-weight: 600;
  }
  .lbl-cta {
    font-weight: 700;
  }
  .sep {
    opacity: 0.4;
  }
</style>
