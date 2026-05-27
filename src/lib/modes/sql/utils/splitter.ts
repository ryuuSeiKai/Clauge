/**
 * Split a SQL text into individual statements.
 * Handles: semicolons, quoted strings, dollar-quoted blocks ($$),
 * single-line comments (--), multi-line comments, MySQL `DELIMITER`
 * directives (so CREATE TRIGGER / CREATE PROCEDURE bodies with embedded
 * `;` don't get split mid-body), and nested constructs.
 */
export function splitSqlStatements(sql: string): string[] {
  const statements: string[] = [];
  let current = '';
  let i = 0;
  const len = sql.length;
  // Active statement terminator. Mutated by MySQL `DELIMITER //` etc.
  let delim = ';';
  // True when we're at the first non-whitespace position of a logical
  // line — DELIMITER directives are only recognised at line start
  // outside of strings/comments.
  let atLineStart = true;

  while (i < len) {
    const ch = sql[i];
    const next = i + 1 < len ? sql[i + 1] : '';

    // MySQL `DELIMITER <token>` at the start of a line. Eats the entire
    // line and switches the active terminator. The directive itself is
    // a client-side construct — never sent to the server.
    if (atLineStart && (ch === 'D' || ch === 'd')) {
      // Peek the rest of the line and try to match `DELIMITER <token>`.
      const eolPeek = sql.indexOf('\n', i);
      const lineSlice = sql.slice(i, eolPeek === -1 ? len : eolPeek);
      const m = lineSlice.match(/^\s*DELIMITER\s+(\S+)\s*$/i);
      if (m) {
        // Flush accumulated current as a statement before switching.
        const flushed = current.trim();
        if (flushed) statements.push(flushed);
        current = '';
        delim = m[1];
        i = eolPeek === -1 ? len : eolPeek + 1;
        atLineStart = true;
        continue;
      }
    }
    if (ch !== ' ' && ch !== '\t' && ch !== '\n' && ch !== '\r') {
      atLineStart = false;
    }
    if (ch === '\n') atLineStart = true;

    // Single-line comment: -- to end of line
    if (ch === '-' && next === '-') {
      const eol = sql.indexOf('\n', i);
      if (eol === -1) {
        current += sql.slice(i);
        i = len;
      } else {
        current += sql.slice(i, eol + 1);
        i = eol + 1;
      }
      continue;
    }

    // Multi-line comment: /* ... */
    if (ch === '/' && next === '*') {
      const end = sql.indexOf('*/', i + 2);
      if (end === -1) {
        current += sql.slice(i);
        i = len;
      } else {
        current += sql.slice(i, end + 2);
        i = end + 2;
      }
      continue;
    }

    // Single-quoted string: '...' with '' escape
    if (ch === "'") {
      current += ch;
      i++;
      while (i < len) {
        if (sql[i] === "'" && i + 1 < len && sql[i + 1] === "'") {
          current += "''";
          i += 2;
        } else if (sql[i] === "'") {
          current += "'";
          i++;
          break;
        } else {
          current += sql[i];
          i++;
        }
      }
      continue;
    }

    // Backtick-quoted identifier (MySQL): `...`
    if (ch === '`') {
      current += ch;
      i++;
      while (i < len && sql[i] !== '`') {
        current += sql[i];
        i++;
      }
      if (i < len) { current += '`'; i++; }
      continue;
    }

    // Double-quoted identifier: "..."
    if (ch === '"') {
      current += ch;
      i++;
      while (i < len) {
        if (sql[i] === '"' && i + 1 < len && sql[i + 1] === '"') {
          current += '""';
          i += 2;
        } else if (sql[i] === '"') {
          current += '"';
          i++;
          break;
        } else {
          current += sql[i];
          i++;
        }
      }
      continue;
    }

    // Dollar-quoted block: $tag$...$tag$ (PostgreSQL)
    if (ch === '$') {
      // Find the end of the opening tag
      const tagEnd = sql.indexOf('$', i + 1);
      if (tagEnd !== -1) {
        const tag = sql.slice(i, tagEnd + 1); // e.g. $$ or $func$
        // Check if it's a valid dollar-quote tag (alphanumeric + underscore only between $)
        const tagContent = tag.slice(1, -1);
        if (tagContent === '' || /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(tagContent)) {
          const closeIdx = sql.indexOf(tag, tagEnd + 1);
          if (closeIdx !== -1) {
            current += sql.slice(i, closeIdx + tag.length);
            i = closeIdx + tag.length;
            continue;
          }
        }
      }
      // Not a dollar-quote, treat as regular character
      current += ch;
      i++;
      continue;
    }

    // Statement separator. Single-char `;` is the common case; multi-char
    // delimiters (set via `DELIMITER //`) are matched against the rest
    // of the buffer.
    if (delim.length === 1 ? ch === delim : sql.startsWith(delim, i)) {
      const stmt = current.trim();
      if (stmt) statements.push(stmt);
      current = '';
      i += delim.length;
      continue;
    }

    current += ch;
    i++;
  }

  // Last statement (may not have trailing ;)
  const last = current.trim();
  if (last) statements.push(last);

  return statements;
}

export interface PositionedStatement {
  /** Trimmed statement text (without the trailing `;`). */
  text: string;
  /** Position of the first non-whitespace character of the statement in the original buffer. */
  from: number;
  /** Position of the trailing `;` (inclusive) — or position of the last non-whitespace char if no trailing `;`. */
  to: number;
}

/**
 * Same parsing rules as `splitSqlStatements` (quotes, comments, dollar-quoting),
 * but tracks the original-buffer position of each statement so callers can find
 * the statement under a cursor. Position semantics:
 *   - `from` is the first non-whitespace char of the statement region
 *   - `to` is the trailing `;` (inclusive) — or the last non-whitespace char if
 *     the statement has no terminating `;`
 * A cursor at position P "belongs to" a statement iff `P >= from && P <= to + 1`
 * (the +1 lets the cursor sit immediately after the `;`).
 */
export function splitSqlStatementsWithPositions(sql: string): PositionedStatement[] {
  const out: PositionedStatement[] = [];
  let regionStart = 0;
  let i = 0;
  const len = sql.length;
  // MySQL DELIMITER state. Same semantics as `splitSqlStatements` —
  // without this, cursor-aware execute inside a CREATE TRIGGER /
  // CREATE PROCEDURE block would select an inner fragment ending at
  // the body's first `;` instead of the full routine.
  let delim = ';';
  let atLineStart = true;

  const emit = (regionEnd: number, hadTerminator: boolean, terminatorLen: number) => {
    const raw = sql.slice(regionStart, regionEnd);
    let leading = 0;
    while (leading < raw.length && /\s/.test(raw[leading])) leading++;
    let trailing = raw.length;
    while (trailing > leading && /\s/.test(raw[trailing - 1])) trailing--;
    if (trailing > leading) {
      out.push({
        text: raw.slice(leading, trailing),
        from: regionStart + leading,
        // `to` points at the last char of the trailing delimiter for
        // multi-char delimiters (so cursor-at-end-of-delim still maps
        // to this statement), or at the last non-whitespace char when
        // there's no terminator.
        to: hadTerminator ? regionEnd + terminatorLen - 1 : regionStart + trailing - 1,
      });
    }
  };

  while (i < len) {
    const ch = sql[i];
    const next = i + 1 < len ? sql[i + 1] : '';

    // DELIMITER directive at line start (MySQL CLI semantics).
    if (atLineStart && (ch === 'D' || ch === 'd')) {
      const eolPeek = sql.indexOf('\n', i);
      const lineSlice = sql.slice(i, eolPeek === -1 ? len : eolPeek);
      const m = lineSlice.match(/^\s*DELIMITER\s+(\S+)\s*$/i);
      if (m) {
        // Emit anything accumulated as a no-terminator statement so its
        // position bounds still close cleanly.
        emit(i, false, 0);
        delim = m[1];
        i = eolPeek === -1 ? len : eolPeek + 1;
        regionStart = i;
        atLineStart = true;
        continue;
      }
    }
    if (ch !== ' ' && ch !== '\t' && ch !== '\n' && ch !== '\r') {
      atLineStart = false;
    }
    if (ch === '\n') atLineStart = true;

    if (ch === '-' && next === '-') {
      const eol = sql.indexOf('\n', i);
      i = eol === -1 ? len : eol + 1;
      continue;
    }
    if (ch === '/' && next === '*') {
      const end = sql.indexOf('*/', i + 2);
      i = end === -1 ? len : end + 2;
      continue;
    }
    if (ch === "'") {
      i++;
      while (i < len) {
        if (sql[i] === "'" && i + 1 < len && sql[i + 1] === "'") { i += 2; }
        else if (sql[i] === "'") { i++; break; }
        else { i++; }
      }
      continue;
    }
    if (ch === '`') {
      i++;
      while (i < len && sql[i] !== '`') i++;
      if (i < len) i++;
      continue;
    }
    if (ch === '"') {
      i++;
      while (i < len) {
        if (sql[i] === '"' && i + 1 < len && sql[i + 1] === '"') { i += 2; }
        else if (sql[i] === '"') { i++; break; }
        else { i++; }
      }
      continue;
    }
    if (ch === '$') {
      const tagEnd = sql.indexOf('$', i + 1);
      if (tagEnd !== -1) {
        const tag = sql.slice(i, tagEnd + 1);
        const tagContent = tag.slice(1, -1);
        if (tagContent === '' || /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(tagContent)) {
          const closeIdx = sql.indexOf(tag, tagEnd + 1);
          if (closeIdx !== -1) {
            i = closeIdx + tag.length;
            continue;
          }
        }
      }
      i++;
      continue;
    }
    // Active statement terminator. Single-char `;` is the common case;
    // multi-char delimiters from `DELIMITER //` are matched by prefix.
    if (delim.length === 1 ? ch === delim : sql.startsWith(delim, i)) {
      emit(i, true, delim.length);
      i += delim.length;
      regionStart = i;
      continue;
    }
    i++;
  }

  emit(len, false, 0);
  return out;
}
