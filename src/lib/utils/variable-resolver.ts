// Resolve {{variable}} placeholders in a string using an env vars map
export function resolveVariables(input: string, vars: Record<string, string>): string {
  return input.replace(/\{\{(\w+)\}\}/g, (match, key) => vars[key] ?? match);
}

// Extract variable names from a string
export function extractVariables(input: string): string[] {
  const matches = input.matchAll(/\{\{(\w+)\}\}/g);
  return [...matches].map(m => m[1]);
}

// Check if a string contains variables
export function hasVariables(input: string): boolean {
  return /\{\{(\w+)\}\}/.test(input);
}
