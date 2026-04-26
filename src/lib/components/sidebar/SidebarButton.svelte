<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    tip = '',
    active = false,
    dotColor = '',
    id = '',
    onclick = () => {},
    children,
  }: {
    tip?: string;
    active?: boolean;
    dotColor?: string;
    id?: string;
    onclick?: () => void;
    children: Snippet;
  } = $props();
</script>

<button
  class="sbi"
  class:on={active}
  data-tip={tip || undefined}
  {id}
  onclick={onclick}
>
  {@render children()}
  {#if dotColor}
    <span class="sbi-dot" style="background:{dotColor}"></span>
  {/if}
</button>

<style>
  .sbi {
    width: 34px;
    height: 34px;
    border-radius: 7px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    position: relative;
    transition: background 0.12s;
  }
  .sbi :global(svg) {
    width: 16px;
    height: 16px;
    stroke: var(--t3);
    fill: none;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: stroke 0.12s;
  }
  .sbi:hover {
    background: var(--b-subtle);
  }
  .sbi:hover :global(svg) {
    stroke: var(--t1);
  }
  .sbi.on :global(svg) {
    stroke: var(--acc);
  }
  .sbi.on {
    background: color-mix(in srgb, var(--acc) 10%, transparent);
  }
  .sbi-dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
  }
  /* tooltip */
  .sbi[data-tip]:hover::after {
    content: attr(data-tip);
    position: absolute;
    left: calc(100% + 10px);
    top: 50%;
    transform: translateY(-50%);
    background: var(--n);
    color: var(--t1);
    font-family: var(--ui);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 5px;
    border: 1px solid var(--b1);
    white-space: nowrap;
    pointer-events: none;
    z-index: 999;
  }
</style>
