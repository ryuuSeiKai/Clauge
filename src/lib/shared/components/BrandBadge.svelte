<script lang="ts">
  // Single badge primitive for connection-type identifiers (Postgres,
  // MongoDB, S3, …). Replaces inline switches in SqlNav / NoSqlNav /
  // ExplorerNav. Renders either a text abbreviation or an icon based on
  // `theme.brandDisplay`; color comes from the brand registry with optional
  // theme overrides.
  import { brandConfig, brandFor, resolveBrandColor, resolveBrandIconPath, resolveBrandAbbreviation, type BrandKey } from '$lib/shared/theme/brands';

  interface Props {
    brand: string;
    /** Outer wrapper size (square). Default 22 — matches old nav badge. */
    size?: number;
    /** Force a display mode for this badge (overrides theme preference).
     *  Used by surfaces that prefer text even when the theme prefers icons. */
    forceDisplay?: 'text' | 'icon';
    /** Optional CSS class for the wrapper. */
    klass?: string;
  }

  let { brand, size = 22, forceDisplay, klass = '' }: Props = $props();

  const descriptor = $derived(brandFor(brand));
  const override = $derived($brandConfig.overrides[brand as BrandKey]);
  const color = $derived(descriptor ? resolveBrandColor(brand as BrandKey, override) : 'var(--t3)');
  const iconPath = $derived(descriptor ? resolveBrandIconPath(brand as BrandKey, override) : '');
  const label = $derived(descriptor ? resolveBrandAbbreviation(brand as BrandKey, override) : (brand ? brand.slice(0, 2).toUpperCase() : '?'));
  const display = $derived(forceDisplay ?? ($brandConfig.display === 'auto' ? 'text' : $brandConfig.display));
</script>

<span
  class="brand-badge {klass}"
  class:brand-badge-icon={display === 'icon' && iconPath}
  style="--brand-color: {color}; --brand-size: {size}px;"
  title={descriptor?.displayName ?? brand}
>
  {#if display === 'icon' && iconPath}
    <svg viewBox="0 0 24 24" width={Math.round(size * 0.7)} height={Math.round(size * 0.7)} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d={iconPath} />
    </svg>
  {:else}
    <span class="brand-badge-text">{label}</span>
  {/if}
</span>

<style>
  .brand-badge {
    /* Pill in text mode (height pinned, width grows with content);
       fixed square in icon mode. Min-width keeps short 2-char abbrevs
       (PG/MY/MG/RD/D1) optically uniform with the longer ones
       (SFTP/AZURE) that need the extra room. */
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    height: var(--brand-size);
    min-width: var(--brand-size);
    padding: 0 6px;
    box-sizing: border-box;
    border-radius: 5px;
    color: var(--brand-color);
    background: color-mix(in srgb, var(--brand-color) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--brand-color) 30%, transparent);
    font-family: var(--ui);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    user-select: none;
    white-space: nowrap;
  }
  .brand-badge.brand-badge-icon {
    width: var(--brand-size);
    padding: 0;
  }
  .brand-badge-text {
    line-height: 1;
  }
</style>
