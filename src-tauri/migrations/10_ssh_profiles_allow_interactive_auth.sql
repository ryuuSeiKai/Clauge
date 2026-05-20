-- Allow auth_type = 'interactive' for keyboard-interactive PAM auth where
-- the SSH server sends one or more prompts (e.g. password + OTP). The
-- existing 'password' type stays plain-password-only — the explicit
-- 'interactive' type drives the prompt UI in the frontend.
--
-- SQLite has no ALTER TABLE for CHECK constraints, so we copy into a new
-- table with the relaxed constraint, drop the original, and rename. Same
-- pattern as migration 7 (add 'agent') and migration 9 (add proxy support).
-- The proxy columns from migration 9 are preserved.
--
-- ssh_known_hosts has FK ON DELETE CASCADE; the DROP empties it. TOFU cache
-- is rebuilt naturally on next connect — same one-time cost as migration 7.

CREATE TABLE ssh_profiles_v10 (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_type TEXT NOT NULL CHECK(auth_type IN ('key','password','agent','interactive')),
    key_path TEXT,
    accent_color TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    jump_profile_id TEXT REFERENCES ssh_profiles_v10(id) ON DELETE SET NULL,
    proxy_command TEXT
);

INSERT INTO ssh_profiles_v10
    (id, name, host, port, username, auth_type, key_path, accent_color,
     last_used_at, created_at, jump_profile_id, proxy_command)
SELECT id, name, host, port, username, auth_type, key_path, accent_color,
       last_used_at, created_at, jump_profile_id, proxy_command
FROM ssh_profiles;

DROP TABLE ssh_profiles;
ALTER TABLE ssh_profiles_v10 RENAME TO ssh_profiles;
