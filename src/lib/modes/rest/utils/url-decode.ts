/**
 * URL-decode a single component, falling back to the raw string on
 * malformed input. `decodeURIComponent` throws a `URIError` whenever
 * a `%` is followed by anything that isn't a valid two-hex-digit
 * pair — e.g. user-pasted URLs containing template placeholders like
 * `%AgentID%`, Mozenda-style `%Job.Status%`, ColdFusion `%foo%`,
 * SQL-Server `%pct%`, Apache `%h`, etc. When that happens inside a
 * Svelte `$derived.by` it crashes the reactive graph silently,
 * leaving the UI in a half-broken state across modes (REST blanks,
 * SQL Run button stays disabled). Wrap every URL-param decode in
 * this helper so a malformed value just round-trips as-is.
 */
export function safeDecodeURIComponent(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}
