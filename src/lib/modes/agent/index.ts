// Public surface for the Agent mode. Importers should prefer this barrel
// over reaching into individual files so internal layout can evolve
// without a fan-out of import edits.

export * from './types';
export * from './stores';

// Raw invoke wrappers are namespaced because store helpers re-export
// names like `loadAgentSessions` that wrap the raw commands with
// runtime-state side effects; pulling both flat would collide.
export * as AgentCommands from './commands';

// AI-facing pieces are namespaced so callers can be explicit about which
// concern they're pulling in.
export * as AgentAI from './ai/prompt';
