// Brand badge registry — single source of truth for SQL / NoSQL / Explorer
// connection-type identifiers (Postgres, MySQL, MongoDB, S3, etc.).
//
// Why this exists: each brand has a defining color (Mongo green, AWS orange)
// that signals "this row is a MongoDB connection" at a glance. Hardcoding
// those in three different nav components meant a soft / off-palette theme
// (e.g. Rosé Pine Moon) couldn't tone them down. This registry lets themes
// override `color`, `icon`, or even hide brand colors entirely via
// `Theme.brandOverrides` + `Theme.brandDisplay`.

import { writable } from 'svelte/store';

export type BrandKey =
  | 'postgresql' | 'mysql' | 'sqlite' | 'clickhouse' | 'd1'
  | 'mongodb' | 'redis'
  | 'sftp' | 'ftp' | 's3' | 'azure_blob';

export interface BrandDescriptor {
  key: BrandKey;
  displayName: string;
  abbreviation: string;
  defaultColor: string;
  /** SVG path data drawn inside a 24×24 viewBox. Stylized representations
   *  — not the real brand logos — so a free build doesn't ship trademarks. */
  iconPath: string;
}

export const BRANDS: Record<BrandKey, BrandDescriptor> = {
  postgresql: {
    key: 'postgresql', displayName: 'PostgreSQL', abbreviation: 'PG',
    defaultColor: '#336791',
    iconPath: 'M12 3c4 0 7 2 7 5v8c0 3-3 5-7 5s-7-2-7-5V8c0-3 3-5 7-5zm0 3c-2.2 0-4 1-4 2v8c0 1 1.8 2 4 2s4-1 4-2V8c0-1-1.8-2-4-2z',
  },
  mysql: {
    key: 'mysql', displayName: 'MySQL', abbreviation: 'MY',
    defaultColor: '#00758F',
    iconPath: 'M4 7c4 0 7 3 8 7M5 9c3 0 6 2 7 6M16 4l4 4-4 4M20 8h-8',
  },
  sqlite: {
    key: 'sqlite', displayName: 'SQLite', abbreviation: 'SL',
    defaultColor: '#909090',
    iconPath: 'M4 6h12l4 4v8a2 2 0 01-2 2H4a2 2 0 01-2-2V8a2 2 0 012-2zm10 0v4h4',
  },
  clickhouse: {
    key: 'clickhouse', displayName: 'ClickHouse', abbreviation: 'CH',
    defaultColor: '#FFCC01',
    iconPath: 'M3 3h2v18H3zM7 3h2v18H7zM11 3h2v18h-2zM15 3h2v18h-2zM19 10h2v4h-2z',
  },
  d1: {
    key: 'd1', displayName: 'Cloudflare D1', abbreviation: 'D1',
    defaultColor: '#F38020',
    iconPath: 'M18 11a4 4 0 00-7.7-1.4A4 4 0 003 12a4 4 0 004 4h11a3 3 0 000-6h-.5z',
  },
  mongodb: {
    key: 'mongodb', displayName: 'MongoDB', abbreviation: 'MG',
    defaultColor: '#00ED64',
    iconPath: 'M12 2c2 4 4 7 4 11s-2 7-4 9c-2-2-4-5-4-9s2-7 4-11zm0 14v6',
  },
  redis: {
    key: 'redis', displayName: 'Redis', abbreviation: 'RD',
    defaultColor: '#DC382D',
    iconPath: 'M3 7l9-4 9 4-9 4-9-4zm0 5l9 4 9-4M3 17l9 4 9-4',
  },
  sftp: {
    key: 'sftp', displayName: 'SFTP', abbreviation: 'SFTP',
    defaultColor: '#06b6d4',
    iconPath: 'M12 2l7 4v6c0 5-3 9-7 10-4-1-7-5-7-10V6l7-4zm-3 10l2 2 4-4',
  },
  ftp: {
    key: 'ftp', displayName: 'FTP', abbreviation: 'FTP',
    defaultColor: '#9ca3af',
    iconPath: 'M3 7a2 2 0 012-2h5l2 2h7a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z',
  },
  s3: {
    key: 's3', displayName: 'Amazon S3', abbreviation: 'S3',
    defaultColor: '#FF9900',
    iconPath: 'M12 2L3 6v10l9 6 9-6V6l-9-4zm0 4l5 2-5 2-5-2 5-2zm-5 4l5 2v8l-5-3v-7zm10 0v7l-5 3v-8l5-2z',
  },
  azure_blob: {
    key: 'azure_blob', displayName: 'Azure Blob', abbreviation: 'Azure',
    defaultColor: '#0078D4',
    iconPath: 'M18 11a4 4 0 00-7.7-1.4A4 4 0 003 12a4 4 0 004 4h11a3 3 0 000-6h-.5z',
  },
};

export interface BrandOverride {
  /** Full color replacement. Wins over `intensity`. */
  color?: string;
  /** Multiplier on the default color via color-mix with white/black.
   *  >1 boosts saturation/lightness, <1 dims. Skipped when `color` is set. */
  intensity?: number;
  /** Replace the SVG path entirely (e.g. ship a real logo in a Pro theme). */
  icon?: string;
  /** Override the 2-3 char abbreviation. */
  abbreviation?: string;
}

export type BrandDisplayMode = 'text' | 'icon' | 'auto';

export interface BrandConfig {
  overrides: Partial<Record<BrandKey, BrandOverride>>;
  display: BrandDisplayMode;
}

/** Reactive — written by `applyTheme()`, read by `BrandBadge.svelte`. */
export const brandConfig = writable<BrandConfig>({ overrides: {}, display: 'text' });

export function brandFor(key: string): BrandDescriptor | undefined {
  return BRANDS[key as BrandKey];
}

/** Resolve a brand's effective color given a (possibly missing) override.
 *  Used by templates that want to apply the color inline. */
export function resolveBrandColor(key: BrandKey, override?: BrandOverride): string {
  const base = BRANDS[key].defaultColor;
  if (override?.color) return override.color;
  if (override?.intensity && override.intensity !== 1) {
    const i = Math.max(0.3, Math.min(1.7, override.intensity));
    // CSS `color-mix` is the cleanest path; <1 mixes toward neutral, >1 toward white.
    const target = i < 1 ? 'var(--c)' : 'white';
    const pct = i < 1 ? Math.round((1 - i) * 100) : Math.round((i - 1) * 100);
    return `color-mix(in srgb, ${base} ${100 - pct}%, ${target})`;
  }
  return base;
}

export function resolveBrandIconPath(key: BrandKey, override?: BrandOverride): string {
  return override?.icon ?? BRANDS[key].iconPath;
}

export function resolveBrandAbbreviation(key: BrandKey, override?: BrandOverride): string {
  return override?.abbreviation ?? BRANDS[key].abbreviation;
}
