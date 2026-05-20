<script lang="ts">
  import type { Environment } from '$lib/types';

  interface Props {
    environments: Environment[];
    selectedId: string | null;
    onselect: (id: string) => void;
    oncreate: () => void;
  }

  let { environments, selectedId, onselect, oncreate }: Props = $props();
</script>

<div class="env-sidebar">
  <div class="env-list">
    {#each environments as env (env.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="env-item"
        class:selected={env.id === selectedId}
        onclick={() => onselect(env.id)}
      >
        <span class="env-dot" style="background: {env.color}"></span>
        <span class="env-name">{env.name}</span>
        {#if env.isDefault === 1}
          <span class="env-default">(default)</span>
        {/if}
      </div>
    {/each}
  </div>

  <button class="env-new-btn" onclick={oncreate}>+ New Environment</button>
</div>

<style>
  .env-sidebar {
    width: 180px;
    border-right: 1px solid var(--b1);
    background: var(--e);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }

  .env-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .env-item {
    padding: 8px 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--t2);
    font-family: var(--ui);
    transition: background 0.1s;
    user-select: none;
    border-left: 2px solid transparent;
  }

  .env-item:hover {
    background: var(--n2);
  }

  .env-item.selected {
    background: var(--n2);
    color: var(--t1);
    border-left-color: var(--acc);
  }

  .env-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .env-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .env-default {
    font-size: 9px;
    color: var(--t3);
    flex-shrink: 0;
  }

  .env-new-btn {
    padding: 9px 14px;
    border: none;
    border-top: 1px solid var(--b1);
    background: transparent;
    color: var(--acc);
    font-size: 12px;
    font-family: var(--ui);
    cursor: pointer;
    transition: background 0.1s;
    flex-shrink: 0;
    margin-top: auto;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .env-new-btn:hover {
    background: var(--n2);
  }
</style>
