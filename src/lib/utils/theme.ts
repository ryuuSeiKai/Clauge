import { invoke } from '@tauri-apps/api/core';

export interface Theme {
  id: string;
  name: string;
  description: string;
  // Surface colors
  sidebar: string;
  nav: string;
  navHeader: string;
  content: string;
  editor: string;
  // Border colors
  border: string;
  borderHover: string;
  borderSubtle: string;
  // Text colors
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  textFaint: string;
  // Modal (always opaque)
  modalBg: string;
  // Status colors (--ok / --warn / --err). Per-theme so a theme can pick a
  // palette that reads well on its own surfaces (e.g. soft pinks on a
  // violet base) instead of falling back to one global dark/light pair.
  ok: string;
  warn: string;
  err: string;
  // Syntax token colors for the homemade JSON / document highlighters
  // (`json-highlight.ts`, NoSQL `DocumentViewer`). Applied as CSS variables
  // on `:root` and consumed by `.str / .num / .key / .boo` rules in
  // `app.css`. `.pu` (punctuation) and `.cm` (comment, when re-introduced)
  // already cascade off `--t2 / --t3`, so only the 4 distinct token types
  // need per-theme overrides.
  tokens: {
    str: string;
    num: string;
    key: string;
    boo: string;
  };
  // Glass properties
  glass: boolean;
  // Optional visual effect class. When set, `applyTheme` adds it to <body>;
  // CSS rules in app.css under that class paint animations or textures
  // behind the app shell (visible through rgba surfaces). Used by themes
  // that go beyond flat colors. Themes that want a static look leave this
  // undefined.
  effectClass?: string;
}

const themes: Record<string, Theme> = {
  'dark-glass': {
    id: 'dark-glass',
    name: 'Dark Glass',
    description: 'Transparent with macOS vibrancy',
    // Surfaces tuned brighter (rgb 22,22,34 vs old 12,12,20) and slightly
    // less translucent (0.92 vs old 0.88) so the theme stays legible in
    // daylight when the desktop behind the vibrancy isn't bright enough
    // to lift the glass on its own. Borders bumped from 0.06/0.10 alpha to
    // 0.10/0.16 so panel separation reads in bright ambient light.
    sidebar: 'rgba(22,22,34,0.92)',
    nav: 'rgba(22,22,34,0.92)',
    navHeader: 'rgba(28,28,42,0.95)',
    content: 'rgba(22,22,34,0.92)',
    editor: 'rgba(22,22,34,0.92)',
    border: 'rgba(255,255,255,0.16)',
    borderHover: 'rgba(255,255,255,0.22)',
    borderSubtle: 'rgba(255,255,255,0.10)',
    textPrimary: '#e8e8f4',
    textSecondary: '#d0d0e4',
    textMuted: '#bcbcd5',
    textFaint: '#8a8ab2',
    modalBg: '#1a1a2c',
    ok: '#1dc880',
    warn: '#f5a623',
    err: '#f04444',
    tokens: { str: '#9ccfd8', num: '#f6c177', key: '#8cb4e0', boo: '#eb6f92' },
    glass: true,
  },
  'dark-solid': {
    id: 'dark-solid',
    name: 'Dark Solid',
    description: 'Opaque dark with purple tints',
    sidebar: '#12121f',
    nav: '#161629',
    navHeader: '#1c1c35',
    content: '#1e1e32',
    editor: '#151528',
    border: '#2d2d48',
    borderHover: '#3e3e62',
    borderSubtle: '#222238',
    textPrimary: '#e4e4f0',
    textSecondary: '#d0d0e4',
    textMuted: '#b0b0c8',
    textFaint: '#7878a0',
    modalBg: '#151528',
    ok: '#1dc880',
    warn: '#f5a623',
    err: '#f04444',
    tokens: { str: '#9ccfd8', num: '#f6c177', key: '#8cb4e0', boo: '#eb6f92' },
    glass: false,
  },
  'midnight': {
    id: 'midnight',
    name: 'Midnight',
    description: 'Pure black, zero distraction',
    sidebar: '#000000',
    nav: '#0a0a0a',
    navHeader: '#121212',
    content: '#0e0e0e',
    editor: '#080808',
    border: '#2a2a2a',
    borderHover: '#3a3a3a',
    borderSubtle: '#1c1c1c',
    textPrimary: '#e8e8e8',
    textSecondary: '#cccccc',
    textMuted: '#999999',
    textFaint: '#666666',
    modalBg: '#0e0e0e',
    ok: '#1dc880',
    warn: '#f5a623',
    err: '#f04444',
    tokens: { str: '#9ccfd8', num: '#f6c177', key: '#8cb4e0', boo: '#eb6f92' },
    glass: false,
  },
  // Rose Pine Moon — palette taken from the official VSCode theme
  // (https://github.com/rose-pine/vscode). Surfaces: Base / Surface /
  // Overlay. Tokens: Gold (string), Rose (number/bool), Foam (property),
  // Pine (success). Cyan is intentionally Rose per Rose Pine convention —
  // it warms the terminal palette so even ANSI output reads on-theme.
  'rose-pine-moon': {
    id: 'rose-pine-moon',
    name: 'Rose Pine Moon',
    description: 'Warm pastel pinks and lavenders',
    sidebar: '#232136',
    nav: '#232136',
    navHeader: '#2a273f',
    content: '#232136',
    editor: '#232136',
    border: '#393552',
    borderHover: '#44415a',
    borderSubtle: '#2a273f',
    textPrimary: '#e0def4',
    textSecondary: '#c5c1d8',
    textMuted: '#908caa',
    textFaint: '#6e6a86',
    modalBg: '#2a273f',
    ok: '#3e8fb0',
    warn: '#f6c177',
    err: '#eb6f92',
    tokens: { str: '#f6c177', num: '#ea9a97', key: '#9ccfd8', boo: '#ea9a97' },
    glass: false,
  },
  // The block below holds themes that aren't currently surfaced in the
  // picker. `getThemes()` iterates this Record, so commenting an entry
  // out of it is the single switch that hides a theme. Their TERMINAL_THEMES
  // entries below, preview swatches in `colors.ts`, and CSS rules in
  // `app.css` remain in place as inert data — uncomment a block here and
  // the theme is fully back, no other edits needed.
  /*
  'carbon-grain': {
    id: 'carbon-grain',
    name: 'Carbon Grain',
    description: 'Warm-neutral dark with subtle film-grain texture',
    sidebar: '#14141a',
    nav: '#14141a',
    navHeader: '#1d1d24',
    content: '#0c0c0e',
    editor: '#14141a',
    border: '#2a2a32',
    borderHover: '#3a3a44',
    borderSubtle: '#1c1c22',
    textPrimary: '#d8d6cf',
    textSecondary: '#b8b6af',
    textMuted: '#8a887f',
    textFaint: '#5a5852',
    modalBg: '#14141a',
    ok: '#84a17d',
    warn: '#d4a373',
    err: '#c3736e',
    tokens: { str: '#b6c294', num: '#d4a373', key: '#a8b7d0', boo: '#d4a373' },
    glass: false,
    effectClass: 'fx-grain',
  },
  'crt-phosphor': {
    id: 'crt-phosphor',
    name: 'CRT Phosphor',
    description: 'Retro green terminal with horizontal scanlines',
    sidebar: '#060a06',
    nav: '#060a06',
    navHeader: '#0a100a',
    content: '#060a06',
    editor: '#050805',
    border: '#1a2a1a',
    borderHover: '#2a4a2a',
    borderSubtle: '#102010',
    textPrimary: '#66ee66',
    textSecondary: '#4ecc4e',
    textMuted: '#3aa83a',
    textFaint: '#237823',
    modalBg: '#0a100a',
    ok: '#00ff80',
    warn: '#ffe066',
    err: '#ff7766',
    tokens: { str: '#b8ee88', num: '#ffe066', key: '#a8eecc', boo: '#ffe066' },
    glass: false,
    effectClass: 'fx-crt',
  },
  'aurora-drift': {
    id: 'aurora-drift',
    name: 'Aurora Drift',
    description: 'Animated aurora gradient drifting behind the surfaces',
    sidebar: 'rgba(14,17,30,0.78)',
    nav: 'rgba(14,17,30,0.78)',
    navHeader: 'rgba(20,23,38,0.85)',
    content: 'rgba(14,17,30,0.72)',
    editor: 'rgba(20,23,38,0.82)',
    border: 'rgba(255,255,255,0.12)',
    borderHover: 'rgba(255,255,255,0.20)',
    borderSubtle: 'rgba(255,255,255,0.06)',
    textPrimary: '#e8e8f4',
    textSecondary: '#cfd0e4',
    textMuted: '#a0a4c0',
    textFaint: '#7078a0',
    modalBg: '#161928',
    ok: '#1dc880',
    warn: '#f5a623',
    err: '#f04444',
    tokens: { str: '#9ccfd8', num: '#f6c177', key: '#8cb4e0', boo: '#eb6f92' },
    glass: false,
    effectClass: 'fx-aurora',
  },
  */
};

export function applyTheme(themeId: string, accentColor?: string) {
  const theme = themes[themeId];
  if (!theme) return;

  const root = document.documentElement;
  root.style.setProperty('--s', theme.sidebar);
  root.style.setProperty('--n', theme.nav);
  root.style.setProperty('--n2', theme.navHeader);
  root.style.setProperty('--c', theme.content);
  root.style.setProperty('--e', theme.editor);
  root.style.setProperty('--b1', theme.border);
  root.style.setProperty('--b2', theme.borderHover);
  root.style.setProperty('--b-subtle', theme.borderSubtle);
  root.style.setProperty('--t1', theme.textPrimary);
  root.style.setProperty('--t2', theme.textSecondary);
  root.style.setProperty('--t3', theme.textMuted);
  root.style.setProperty('--t4', theme.textFaint);
  root.style.setProperty('--modal-bg', theme.modalBg);

  if (accentColor) {
    root.style.setProperty('--acc', accentColor);
  }

  // Status colors are now per-theme. The legacy `light-mode` body class is
  // intentionally not toggled here — the only theme that set it was removed,
  // and any `.light-mode` rules left in app.css are dormant. Re-introduce
  // the toggle if a light theme is added back later.
  root.style.setProperty('--ok', theme.ok);
  root.style.setProperty('--warn', theme.warn);
  root.style.setProperty('--err', theme.err);

  // Syntax token colors — read by `.str / .num / .key / .boo` in app.css
  // (the homemade JSON + document highlighters). Per-theme means each
  // theme owns its own JSON-viewer token palette.
  root.style.setProperty('--syntax-str', theme.tokens.str);
  root.style.setProperty('--syntax-num', theme.tokens.num);
  root.style.setProperty('--syntax-key', theme.tokens.key);
  root.style.setProperty('--syntax-boo', theme.tokens.boo);

  // Visual-effect class (e.g. `fx-aurora`). Strip any prior fx-* class
  // before applying this theme's so swaps clean up correctly. Effect CSS
  // lives in `app.css` keyed off the body class — pure CSS, no JS needed.
  for (const cls of Array.from(document.body.classList)) {
    if (cls.startsWith('fx-')) document.body.classList.remove(cls);
  }
  if (theme.effectClass) document.body.classList.add(theme.effectClass);

  // Glass-specific: add backdrop-filter class and set vibrancy
  if (theme.glass) {
    document.body.classList.add('glass-mode');
    setVibrancy('sidebar');
  } else {
    document.body.classList.remove('glass-mode');
    setVibrancy('none');
  }
}

export function getThemes() { return Object.values(themes); }
export function getTheme(id: string) { return themes[id]; }

export async function setVibrancy(material: string) {
  try {
    await invoke('set_vibrancy', { material });
  } catch (e) {
    console.warn('Vibrancy not supported:', e);
  }
}

// xterm.js terminal themes matched to each app theme
export const TERMINAL_THEMES: Record<string, Record<string, string>> = {
  'dark-glass': {
    background: '#0d0d18',
    foreground: '#e8e8f4',
    cursor: '#6366f1',
    cursorAccent: '#0d0d18',
    selectionBackground: 'rgba(99,102,241,0.3)',
    black: '#484858', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e6edf3',
    brightBlack: '#6e7681', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  'dark-solid': {
    background: '#12121f',
    foreground: '#e4e4f0',
    cursor: '#6366f1',
    cursorAccent: '#12121f',
    selectionBackground: 'rgba(99,102,241,0.3)',
    black: '#484858', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e6edf3',
    brightBlack: '#6e7681', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  'midnight': {
    background: '#000000',
    foreground: '#e8e8e8',
    cursor: '#6366f1',
    cursorAccent: '#000000',
    selectionBackground: 'rgba(99,102,241,0.25)',
    black: '#3a3a3a', red: '#ff7b72', green: '#3fb950', yellow: '#d29922',
    blue: '#58a6ff', magenta: '#d2a8ff', cyan: '#56d4dd', white: '#e8e8e8',
    brightBlack: '#666666', brightRed: '#ffa198', brightGreen: '#56d364', brightYellow: '#e3b341',
    brightBlue: '#79c0ff', brightMagenta: '#d2a8ff', brightCyan: '#76e4f7', brightWhite: '#ffffff',
  },
  // Rose Pine Moon terminal palette per the official VSCode theme
  // `terminal.ansi*` keys. Note: cyan maps to Rose (#ea9a97), not Foam —
  // that's the Rose Pine convention (warm-cyan).
  'rose-pine-moon': {
    background: '#232136',
    foreground: '#e0def4',
    cursor: '#e0def4',
    cursorAccent: '#232136',
    selectionBackground: 'rgba(129,124,156,0.3)',
    black: '#393552', red: '#eb6f92', green: '#3e8fb0', yellow: '#f6c177',
    blue: '#9ccfd8', magenta: '#c4a7e7', cyan: '#ea9a97', white: '#e0def4',
    brightBlack: '#6e6a86', brightRed: '#eb6f92', brightGreen: '#3e8fb0', brightYellow: '#f6c177',
    brightBlue: '#9ccfd8', brightMagenta: '#c4a7e7', brightCyan: '#ea9a97', brightWhite: '#e0def4',
  },
  'carbon-grain': {
    background: '#0c0c0e',
    foreground: '#d8d6cf',
    cursor: '#d8d6cf',
    cursorAccent: '#0c0c0e',
    selectionBackground: 'rgba(168,183,208,0.22)',
    black: '#2a2a32', red: '#c3736e', green: '#84a17d', yellow: '#d4a373',
    blue: '#a8b7d0', magenta: '#b39bbf', cyan: '#8fb3b1', white: '#b8b6af',
    brightBlack: '#5a5852', brightRed: '#d68a85', brightGreen: '#a4bb9c', brightYellow: '#e6bd91',
    brightBlue: '#bdcae0', brightMagenta: '#c8b4d0', brightCyan: '#a9c8c6', brightWhite: '#e8e6df',
  },
  'crt-phosphor': {
    background: '#050805',
    foreground: '#66ee66',
    cursor: '#88ff88',
    cursorAccent: '#050805',
    selectionBackground: 'rgba(102,238,102,0.22)',
    black: '#1a2a1a', red: '#ff7766', green: '#66ee66', yellow: '#ffe066',
    blue: '#a8eecc', magenta: '#88ddaa', cyan: '#c0ffc0', white: '#b8ee88',
    brightBlack: '#3aa83a', brightRed: '#ff9988', brightGreen: '#88ff88', brightYellow: '#ffefa8',
    brightBlue: '#c0ffe0', brightMagenta: '#a8eecc', brightCyan: '#e0ffe0', brightWhite: '#ffffff',
  },
  'aurora-drift': {
    background: '#0e111e',
    foreground: '#e8e8f4',
    cursor: '#9aa6ff',
    cursorAccent: '#0e111e',
    selectionBackground: 'rgba(124,92,248,0.25)',
    black: '#3b3b55', red: '#ff7b9c', green: '#3fdda6', yellow: '#f6c177',
    blue: '#7aa2f7', magenta: '#c4a7e7', cyan: '#7fdcd0', white: '#cfd0e4',
    brightBlack: '#5a5a78', brightRed: '#ffa3bd', brightGreen: '#76e4be', brightYellow: '#ffd589',
    brightBlue: '#9cb8ff', brightMagenta: '#d9c5f0', brightCyan: '#a4ebe2', brightWhite: '#ffffff',
  },
};

/** Get xterm theme for a given app theme, with accent color as cursor */
export function getTerminalTheme(themeId: string, accentColor?: string): Record<string, string> {
  const termTheme = TERMINAL_THEMES[themeId] || TERMINAL_THEMES['dark-glass'];
  if (accentColor) {
    return { ...termTheme, cursor: accentColor };
  }
  return { ...termTheme };
}

// Method colors for HTTP methods
export const METHOD_COLORS: Record<string, { color: string; bg: string }> = {
  GET:    { color: '#60a5fa', bg: '#162640' },
  POST:   { color: '#34d399', bg: '#0d2818' },
  PUT:    { color: '#fbbf24', bg: '#1c1808' },
  PATCH:  { color: '#c4b5fd', bg: '#1e162e' },
  DELETE: { color: '#f87171', bg: '#2a1010' },
};
