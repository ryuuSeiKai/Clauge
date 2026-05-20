-- Rename existing board columns to match the corrected default naming.
--
-- Earlier builds seeded boards with "In Review" (active work) and
-- "Review" (safety gate before Done) — confusing because both contained
-- the word "Review". The new convention matches standard kanban usage:
--   "In Progress" — active work
--   "In Review"   — safety gate the user clears to Done
--
-- Order matters: rename the old "In Review" first (so the second UPDATE
-- doesn't accidentally collide with it).
UPDATE workspace_board_columns
   SET name = 'In Progress'
 WHERE name = 'In Review';

UPDATE workspace_board_columns
   SET name = 'In Review'
 WHERE name = 'Review';
