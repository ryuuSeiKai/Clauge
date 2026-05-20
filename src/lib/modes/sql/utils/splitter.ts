/**
 * Split a SQL text into individual statements.
 * Handles: semicolons, quoted strings, dollar-quoted blocks ($$),
 * single-line comments (--), multi-line comments, and nested constructs.
 */
export function splitSqlStatements(sql: string): string[] {
  const statements: string[] = [];
  let current = '';
  let i = 0;
  const len = sql.length;

  while (i < len) {
    const ch = sql[i];
    const next = i + 1 < len ? sql[i + 1] : '';

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

    // Statement separator
    if (ch === ';') {
      const stmt = current.trim();
      if (stmt) statements.push(stmt);
      current = '';
      i++;
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
