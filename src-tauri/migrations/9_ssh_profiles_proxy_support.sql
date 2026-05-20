-- Add ProxyJump (jump_profile_id) and ProxyCommand (proxy_command) support
-- to ssh_profiles. Both nullable; NULL = no proxy.
--
-- jump_profile_id is a self-FK referencing another row in ssh_profiles,
-- enabling profile chains (A → B → C). ON DELETE SET NULL clears the
-- pointer if the jump host is deleted. Cycle protection is enforced at
-- connect time in ssh_session.rs (visited HashSet) since SQLite cannot
-- detect arbitrary-depth cycles via constraint alone.
--
-- proxy_command holds the raw OpenSSH-style template (with %h, %p, %r
-- placeholders). At connect time it's tokenized via shell-words and
-- argv[0] is spawned directly — NO /bin/sh invocation. Keeps
-- cross-platform behavior identical and avoids shell-escape issues.
--
-- If both columns are populated on the same row, ProxyCommand takes
-- precedence at connect time (matches OpenSSH ssh_config(5) spec).

ALTER TABLE ssh_profiles ADD COLUMN jump_profile_id TEXT
  REFERENCES ssh_profiles(id) ON DELETE SET NULL;

ALTER TABLE ssh_profiles ADD COLUMN proxy_command TEXT;
