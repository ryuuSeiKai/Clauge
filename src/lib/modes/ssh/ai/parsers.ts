// Shape parsers for SSH command output. The AI picks a `render_as` hint when
// it calls execute_shell; parseShell() turns the raw captured stdout into a
// structured shape the UI can render. Any parser failure falls back to `raw`
// so the user always sees something useful.

export type RenderAs = 'auto' | 'table' | 'kv' | 'status' | 'log' | 'json' | 'diff' | 'raw';

export interface ParsedTable {
  headers: string[];
  rows: string[][];
  totalRows: number;
  truncated: boolean;
}

export interface ParsedKV {
  pairs: { key: string; value: string }[];
}

export interface ParsedLog {
  totalLines: number;
  totalBytes: number;
  headLines: string[];
  tailLines: string[];
  errorLines: string[];
}

export interface ParsedJson {
  pretty: string;
  valid: boolean;
  bytes: number;
}

export interface ParsedDiff {
  files: { path: string; added: number; removed: number }[];
  preview: string;
  totalLines: number;
}

export interface ParsedStatus {
  value: string;
}

export interface ParsedRaw {
  text: string;
  bytes: number;
}

export type ParsedShape =
  | { kind: 'table'; data: ParsedTable }
  | { kind: 'kv'; data: ParsedKV }
  | { kind: 'log'; data: ParsedLog }
  | { kind: 'json'; data: ParsedJson }
  | { kind: 'diff'; data: ParsedDiff }
  | { kind: 'status'; data: ParsedStatus }
  | { kind: 'raw'; data: ParsedRaw };

const LOG_HEAD_LINES = 20;
const LOG_TAIL_LINES = 20;
const LOG_ERROR_SAMPLE = 6;
const TABLE_ROW_CAP = 200;
const KV_PAIR_CAP = 100;
const DIFF_PREVIEW_LINES = 80;

const ERROR_RE = /\b(error|err|fail|failed|fatal|panic|denied|refused|exception|traceback)\b/i;

export function parseShell(raw: string, hint: RenderAs): ParsedShape {
  const text = raw ?? '';
  const trimmed = text.trim();
  if (!trimmed) {
    return { kind: 'raw', data: { text: '', bytes: 0 } };
  }

  const effective: RenderAs = hint === 'auto' ? detect(trimmed) : hint;

  switch (effective) {
    case 'table':   return tryTable(trimmed) ?? rawShape(text);
    case 'kv':      return tryKV(trimmed) ?? rawShape(text);
    case 'log':     return logShape(text);
    case 'json':    return tryJson(trimmed) ?? rawShape(text);
    case 'diff':    return tryDiff(trimmed) ?? rawShape(text);
    case 'status':  return statusShape(trimmed);
    case 'raw':
    default:        return rawShape(text);
  }
}

function detect(trimmed: string): RenderAs {
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) return 'json';
  if (/^(diff --git|---\s|\+\+\+\s|@@\s)/m.test(trimmed)) return 'diff';
  if (!trimmed.includes('\n')) return 'status';
  return 'raw';
}

function rawShape(text: string): ParsedShape {
  return { kind: 'raw', data: { text, bytes: byteLen(text) } };
}

function statusShape(trimmed: string): ParsedShape {
  const firstLine = trimmed.split('\n')[0].trim();
  return { kind: 'status', data: { value: firstLine } };
}

function logShape(text: string): ParsedShape {
  const lines = text.split('\n');
  const nonEmptyEnd = lines[lines.length - 1] === '' ? lines.length - 1 : lines.length;
  const head = lines.slice(0, Math.min(LOG_HEAD_LINES, nonEmptyEnd));
  const tail = nonEmptyEnd > LOG_HEAD_LINES + LOG_TAIL_LINES
    ? lines.slice(nonEmptyEnd - LOG_TAIL_LINES, nonEmptyEnd)
    : [];
  const errors: string[] = [];
  for (let i = 0; i < nonEmptyEnd && errors.length < LOG_ERROR_SAMPLE; i++) {
    if (ERROR_RE.test(lines[i])) errors.push(lines[i]);
  }
  return {
    kind: 'log',
    data: {
      totalLines: nonEmptyEnd,
      totalBytes: byteLen(text),
      headLines: head,
      tailLines: tail,
      errorLines: errors,
    },
  };
}

function tryTable(trimmed: string): ParsedShape | null {
  const lines = trimmed.split('\n').filter((l) => l.length > 0);
  if (lines.length < 2) return null;

  const splitMulti = (s: string) => s.trim().split(/\s{2,}/);
  let headers = splitMulti(lines[0]);
  let rows: string[][];

  if (headers.length >= 2 && lines.slice(1, Math.min(4, lines.length)).every((l) => splitMulti(l).length >= 2)) {
    rows = lines.slice(1).map((l) => splitMulti(l));
  } else {
    headers = lines[0].trim().split(/\s+/);
    if (headers.length < 2) return null;
    rows = lines.slice(1).map((l) => splitToN(l, headers.length));
  }

  if (rows.length === 0) return null;

  const totalRows = rows.length;
  const truncated = rows.length > TABLE_ROW_CAP;
  if (truncated) rows = rows.slice(0, TABLE_ROW_CAP);

  return { kind: 'table', data: { headers, rows, totalRows, truncated } };
}

function splitToN(line: string, n: number): string[] {
  const parts = line.trim().split(/\s+/);
  if (parts.length <= n) return parts;
  const head = parts.slice(0, n - 1);
  const tail = parts.slice(n - 1).join(' ');
  return [...head, tail];
}

const KV_RE = /^([\w.\-/]+)\s*[:=]\s*(.+?)\s*$/;

function tryKV(trimmed: string): ParsedShape | null {
  const lines = trimmed.split('\n').filter((l) => l.trim().length > 0);
  if (lines.length === 0) return null;
  const pairs: { key: string; value: string }[] = [];
  let matched = 0;
  for (const line of lines) {
    const m = line.match(KV_RE);
    if (m) {
      matched++;
      if (pairs.length < KV_PAIR_CAP) pairs.push({ key: m[1], value: m[2] });
    }
  }
  if (matched / lines.length < 0.6) return null;
  return { kind: 'kv', data: { pairs } };
}

function tryJson(trimmed: string): ParsedShape | null {
  try {
    const obj = JSON.parse(trimmed);
    const pretty = JSON.stringify(obj, null, 2);
    return { kind: 'json', data: { pretty, valid: true, bytes: byteLen(pretty) } };
  } catch {
    return null;
  }
}

function tryDiff(trimmed: string): ParsedShape | null {
  const lines = trimmed.split('\n');
  if (!lines.some((l) => l.startsWith('@@') || l.startsWith('+++') || l.startsWith('--- '))) return null;

  const files: { path: string; added: number; removed: number }[] = [];
  let current: { path: string; added: number; removed: number } | null = null;

  for (const l of lines) {
    if (l.startsWith('+++ ')) {
      const path = l.slice(4).replace(/^b\//, '').trim();
      current = { path, added: 0, removed: 0 };
      files.push(current);
    } else if (current) {
      if (l.startsWith('+') && !l.startsWith('+++')) current.added++;
      else if (l.startsWith('-') && !l.startsWith('---')) current.removed++;
    }
  }
  if (files.length === 0) return null;

  const preview = lines.slice(0, DIFF_PREVIEW_LINES).join('\n');
  return { kind: 'diff', data: { files, preview, totalLines: lines.length } };
}

function byteLen(s: string): number {
  try { return new TextEncoder().encode(s).length; } catch { return s.length; }
}
