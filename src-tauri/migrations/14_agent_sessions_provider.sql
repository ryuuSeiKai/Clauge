-- Multi-CLI agent support: each `agent_sessions` row now records which
-- coding-assistant CLI it talks to. Existing rows are all implicitly
-- Claude — `DEFAULT 'claude'` preserves that without a backfill step.
-- New providers (codex, opencode, ...) are persisted as their stable
-- `CliRunner::id()` string and dispatched through
-- `src/shared/cli/registry.rs::runner_for`.

ALTER TABLE agent_sessions
    ADD COLUMN provider TEXT NOT NULL DEFAULT 'claude';
