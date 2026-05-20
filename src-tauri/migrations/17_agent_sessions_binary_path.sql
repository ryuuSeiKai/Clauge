-- Per-session override for the CLI binary. When NULL, the spawn path
-- falls back to the standard `find_binary(provider.binary_name())`
-- lookup against $PATH. Set this when the user has installed the
-- agent CLI in a non-standard location (custom prefix, version
-- pinning per project, etc).
ALTER TABLE agent_sessions ADD COLUMN binary_path TEXT;
