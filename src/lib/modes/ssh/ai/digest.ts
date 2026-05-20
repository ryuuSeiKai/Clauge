// Compresses captured SSH output into a compact summary for the model.
// The full text stays in the UI card; only this short digest flows back
// as the tool_result so chat history doesn't balloon turn-over-turn.

import type { ParsedShape } from './parsers';

const HARD_CAP_CHARS = 4000;
const TABLE_ROWS_FOR_MODEL = 30;
const TABLE_CELL_CHARS = 80;
const LOG_HEAD_FOR_MODEL = 20;
const LOG_TAIL_FOR_MODEL = 20;
const KV_PAIRS_FOR_MODEL = 40;
const JSON_BYTES_INLINE = 1500;
const DIFF_LINES_FOR_MODEL = 50;

export function digestForModel(parsed: ParsedShape, raw: string): string {
  const out = build(parsed, raw);
  if (out.length <= HARD_CAP_CHARS) return out;
  return out.slice(0, HARD_CAP_CHARS) + `\n[… digest truncated at ${HARD_CAP_CHARS} chars]`;
}

function build(parsed: ParsedShape, raw: string): string {
  switch (parsed.kind) {
    case 'status': return digestStatus(parsed.data);
    case 'kv':     return digestKV(parsed.data);
    case 'table':  return digestTable(parsed.data);
    case 'log':    return digestLog(parsed.data);
    case 'json':   return digestJson(parsed.data);
    case 'diff':   return digestDiff(parsed.data);
    case 'raw':    return digestRaw(raw);
  }
}

function digestStatus(d: { value: string }): string {
  return d.value;
}

function digestKV(d: { pairs: { key: string; value: string }[] }): string {
  const shown = d.pairs.slice(0, KV_PAIRS_FOR_MODEL);
  const lines = shown.map((p) => `${p.key}=${truncate(p.value, 200)}`);
  const more = d.pairs.length - shown.length;
  if (more > 0) lines.push(`[… ${more} more pairs not shown]`);
  return lines.join('\n');
}

function digestTable(d: { headers: string[]; rows: string[][]; totalRows: number; truncated: boolean }): string {
  const rows = d.rows.slice(0, TABLE_ROWS_FOR_MODEL);
  const all = [d.headers, ...rows].map((r) => r.map((c) => truncate(c, TABLE_CELL_CHARS)).join('\t'));
  const omitted = d.totalRows - rows.length;
  const lines = [...all];
  if (omitted > 0) lines.push(`[… ${omitted} more rows of ${d.totalRows} total]`);
  return lines.join('\n');
}

function digestLog(d: { totalLines: number; totalBytes: number; headLines: string[]; tailLines: string[]; errorLines: string[] }): string {
  const parts: string[] = [];
  parts.push(`[log: ${d.totalLines} lines, ${formatBytes(d.totalBytes)}]`);
  if (d.errorLines.length > 0) {
    parts.push('--- error-matching lines ---');
    parts.push(...d.errorLines);
  }
  if (d.tailLines.length === 0) {
    parts.push(...d.headLines.slice(0, LOG_HEAD_FOR_MODEL));
  } else {
    parts.push('--- first lines ---');
    parts.push(...d.headLines.slice(0, LOG_HEAD_FOR_MODEL));
    parts.push(`[… ${d.totalLines - d.headLines.length - d.tailLines.length} lines omitted …]`);
    parts.push('--- last lines ---');
    parts.push(...d.tailLines.slice(-LOG_TAIL_FOR_MODEL));
  }
  return parts.join('\n');
}

function digestJson(d: { pretty: string; bytes: number }): string {
  if (d.bytes <= JSON_BYTES_INLINE) return d.pretty;
  return d.pretty.slice(0, JSON_BYTES_INLINE) + `\n[… JSON truncated, ${formatBytes(d.bytes)} total]`;
}

function digestDiff(d: { files: { path: string; added: number; removed: number }[]; preview: string; totalLines: number }): string {
  const summary = d.files.map((f) => `${f.path}  +${f.added} -${f.removed}`).join('\n');
  const previewLines = d.preview.split('\n').slice(0, DIFF_LINES_FOR_MODEL).join('\n');
  return `--- file summary ---\n${summary}\n--- preview ---\n${previewLines}`;
}

function digestRaw(raw: string): string {
  const text = raw ?? '';
  if (text.length <= HARD_CAP_CHARS - 100) return text;
  const head = text.slice(0, 3000);
  const tail = text.slice(-800);
  const omitted = text.length - head.length - tail.length;
  return `${head}\n[… ${omitted} chars omitted …]\n${tail}`;
}

function truncate(s: string, max: number): string {
  if (s.length <= max) return s;
  return s.slice(0, max - 1) + '…';
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}
