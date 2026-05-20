<script lang="ts">
  // Workspace main panel — Co-workers grid. Shows every persona the
  // user has set up; clicking a card opens it for edit. The first tile
  // is the "+ New coworker" affordance.
  //
  // Empty state explains the concept (one paragraph, no fluff): a
  // coworker is a named persona that drives an agent under the hood.
  // Tag them on cards instead of generic @claude.

  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { coworkers, loadCoworkers } from '../stores';
  import { upgradeModalOpen } from '$lib/stores/cloud';
  import CoworkerAvatar from './CoworkerAvatar.svelte';
  import CoworkerModal from './CoworkerModal.svelte';
  import type { WorkspaceCoworker } from '../types';

  let modalOpen = $state(false);
  let editing = $state<WorkspaceCoworker | null>(null);

  // Re-fetch when the Rust ProStateManager applies the cap (sign-out,
  // downgrade) or re-enables on upgrade. The events carry no per-coworker
  // diff; cheapest correct option is to re-read the list. Without this,
  // the user keeps seeing a stale "all 4 active" view after the hook ran.
  let unlistenLapsed: UnlistenFn | null = null;
  let unlistenUpgraded: UnlistenFn | null = null;

  onMount(async () => {
    await loadCoworkers();
    unlistenLapsed = await listen('cloud:plan_lapsed', () => { loadCoworkers(); });
    unlistenUpgraded = await listen('cloud:plan_upgraded', () => { loadCoworkers(); });
  });
  onDestroy(() => {
    unlistenLapsed?.();
    unlistenUpgraded?.();
  });

  function openNew() {
    editing = null;
    modalOpen = true;
  }
  function openTile(cw: WorkspaceCoworker) {
    // Soft-disabled coworkers belong to a previously-Pro account that
    // downgraded. Clicking them should sell the upgrade, not let the
    // user edit a personality that's about to be ignored anyway.
    if (cw.disabledAt != null) {
      upgradeModalOpen.set(true);
      return;
    }
    editing = cw;
    modalOpen = true;
  }
</script>

<div class="cv">
  <header class="cv-head">
    <div class="cv-head-row">
      <span class="cv-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="9" cy="8" r="3.5"/>
          <path d="M2.5 19a6.5 6.5 0 0 1 13 0"/>
          <circle cx="17" cy="6" r="2.6"/>
          <path d="M14 13a4.5 4.5 0 0 1 8.5 2"/>
        </svg>
      </span>
      <h1 class="cv-title">Co-workers</h1>
      <span class="cv-count">{$coworkers.length} {$coworkers.length === 1 ? 'persona' : 'personas'}</span>
      <button class="cv-new" onclick={openNew}>+ New coworker</button>
    </div>
    <p class="cv-sub">Agent personas with custom roles and instructions. Assign them to cards to get focused, consistent responses.</p>
  </header>

  <div class="cv-body">
    {#if $coworkers.length === 0}
      <div class="cv-empty">
        <svg viewBox="0 0 24 24" width="42" height="42" fill="none" stroke="var(--t4)" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="9" cy="8" r="3.5"/><path d="M2.5 19a6.5 6.5 0 0 1 13 0"/><circle cx="17" cy="6" r="2.6"/><path d="M14 13a4.5 4.5 0 0 1 8.5 2"/>
        </svg>
        <h3>No coworkers yet</h3>
        <p>Create a persona, give it a role and instructions, then assign it to a card.</p>
        <button class="cv-cta" onclick={openNew}>+ Create your first coworker</button>
      </div>
    {:else}
      <div class="cv-grid">
        <!-- "Add" tile -->
        <button class="cv-tile cv-tile-add" onclick={openNew}>
          <span class="cv-tile-add-plus">+</span>
          <span class="cv-tile-add-label">New coworker</span>
        </button>

        {#each $coworkers as cw (cw.id)}
          {@const locked = cw.disabledAt != null}
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <button
            class="cv-tile"
            class:cv-tile-locked={locked}
            onclick={() => openTile(cw)}
            title={locked
              ? `Locked — upgrade to Pro to re-enable @${cw.name}`
              : `Edit @${cw.name}`}
          >
            <div class="cv-avatar-wrap">
              <CoworkerAvatar seed={cw.avatarSeed} style={cw.avatarStyle} size={64} ring />
              {#if locked}
                <span class="cv-pro-badge" aria-hidden="true">
                  <svg width="9" height="9" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 2l2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6L12 2z" />
                  </svg>
                  PRO
                </span>
              {/if}
            </div>
            <div class="cv-tile-name">
              @{cw.name}
            </div>
            {#if cw.role}
              <div class="cv-tile-role">{cw.role}</div>
            {/if}
            {#if locked}
              <div class="cv-tile-locked-cta">Upgrade to re-enable</div>
            {:else if cw.systemPrompt}
              <div class="cv-tile-prompt">{cw.systemPrompt}</div>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<CoworkerModal bind:show={modalOpen} existing={editing} />

<style>
  .cv { flex: 1; display: flex; flex-direction: column; min-height: 0; overflow: hidden; }
  .cv-head {
    flex-shrink: 0;
    padding: 16px 22px 14px;
    border-bottom: 1px solid var(--b1);
    background: var(--n2);
  }
  .cv-head-row { display: flex; align-items: center; gap: 10px; }
  .cv-icon { color: var(--acc); display: inline-flex; }
  .cv-title { margin: 0; font-size: 16px; font-weight: 600; color: var(--t1); font-family: var(--ui); }
  .cv-count { font-size: 11px; color: var(--t3); font-family: var(--ui); }
  .cv-new {
    margin-left: auto;
    height: 28px; padding: 0 14px; border-radius: 6px;
    border: none; background: var(--acc); color: #fff;
    font-family: var(--ui); font-size: 12px; font-weight: 600;
    cursor: default;
  }
  .cv-new:hover { opacity: 0.9; }
  .cv-sub { margin: 8px 0 0; font-size: 11.5px; color: var(--t3); font-family: var(--ui); line-height: 1.55; max-width: 720px; }
  .cv-sub strong { color: var(--t2); font-weight: 600; }

  .cv-body { flex: 1; overflow-y: auto; min-height: 0; padding: 18px 22px 28px; }

  .cv-empty {
    display: flex; flex-direction: column; align-items: center; gap: 10px;
    padding: 60px 40px; color: var(--t3); text-align: center;
  }
  .cv-empty h3 { margin: 6px 0 0; font-size: 14px; font-weight: 600; color: var(--t2); font-family: var(--ui); }
  .cv-empty p { margin: 0; font-size: 12px; color: var(--t3); font-family: var(--ui); max-width: 420px; line-height: 1.6; }
  .cv-cta {
    margin-top: 6px;
    padding: 8px 18px; border-radius: 8px;
    border: 1px solid var(--acc);
    background: color-mix(in srgb, var(--acc) 18%, transparent);
    color: var(--t1);
    font-size: 12.5px; font-family: var(--ui); font-weight: 500;
    cursor: default;
  }
  .cv-cta:hover { background: color-mix(in srgb, var(--acc) 28%, transparent); }

  .cv-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }
  .cv-tile {
    border: 1px solid var(--b1);
    background: var(--surface-card);
    border-radius: 10px;
    padding: 16px 14px 14px;
    display: flex; flex-direction: column; align-items: flex-start; gap: 8px;
    cursor: default;
    text-align: left;
    transition: border-color 0.12s, background 0.12s;
  }
  .cv-tile:hover { border-color: var(--acc); background: var(--surface-hover); }
  .cv-tile-name {
    font-family: var(--ui);
    font-size: 13.5px;
    font-weight: 600;
    color: var(--t1);
  }
  .cv-tile-role {
    font-family: var(--ui);
    font-size: 11px;
    color: var(--acc);
    font-weight: 500;
  }
  .cv-tile-prompt {
    font-family: var(--ui);
    font-size: 11px;
    color: var(--t3);
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .cv-tile-add {
    border: 1px dashed var(--b1);
    background: transparent;
    color: var(--t3);
    align-items: center;
    justify-content: center;
    min-height: 156px;
  }
  .cv-tile-add:hover {
    border-color: var(--acc);
    color: var(--t1);
    background: color-mix(in srgb, var(--acc) 4%, transparent);
  }
  .cv-tile-add-plus { font-size: 28px; line-height: 1; font-weight: 300; }
  .cv-tile-add-label { font-family: var(--ui); font-size: 12px; }

  /* Locked tile = previously-Pro coworker, soft-disabled after downgrade.
     Stays clickable (opens UpgradeModal) instead of passive grey. */
  .cv-tile-locked {
    cursor: pointer;
    border-style: dashed;
    border-color: color-mix(in srgb, var(--acc) 40%, var(--b1));
  }
  .cv-tile-locked .cv-tile-name,
  .cv-tile-locked .cv-tile-role {
    opacity: 0.55;
  }
  .cv-tile-locked:hover {
    border-color: var(--acc);
    background: color-mix(in srgb, var(--acc) 6%, var(--surface-card));
    transform: translateY(-1px);
  }
  .cv-avatar-wrap {
    position: relative;
    display: inline-flex;
  }
  .cv-tile-locked .cv-avatar-wrap {
    filter: grayscale(0.5);
    opacity: 0.78;
  }
  .cv-pro-badge {
    position: absolute;
    top: -4px;
    right: -8px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 2px 6px 2px 4px;
    font-family: var(--ui);
    font-size: 9px;
    font-weight: 800;
    letter-spacing: 0.08em;
    color: #fff;
    background: linear-gradient(120deg, var(--acc), color-mix(in srgb, var(--acc) 70%, #fff));
    border-radius: 999px;
    box-shadow:
      0 2px 6px color-mix(in srgb, var(--acc) 35%, transparent),
      0 0 0 1.5px var(--surface-card);
    pointer-events: none;
  }
  .cv-pro-badge svg {
    filter: drop-shadow(0 0 2px rgba(255, 255, 255, 0.6));
  }
  .cv-tile-locked-cta {
    margin-top: 2px;
    font-family: var(--ui);
    font-size: 11px;
    font-weight: 600;
    color: var(--acc);
    letter-spacing: 0.01em;
  }
  .cv-tile-name { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
</style>
