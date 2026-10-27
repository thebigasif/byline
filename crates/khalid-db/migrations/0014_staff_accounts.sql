-- Staff accounts as khalid-auth keeps them (ADR "staff accounts in
-- khalid-auth"). users and sessions come from 0003; this adds what signing
-- in needs. Migrations are forward-only (RULES.md BL-DB-2).

-- A site has one owner; a transfer changes it in one write.
CREATE UNIQUE INDEX users_one_owner ON users (site_id) WHERE role = 'owner';

-- When a session was last used, so one unused for 12 hours ends. NULL
-- means not since it began.
ALTER TABLE sessions ADD COLUMN last_seen_at TEXT
    CHECK (last_seen_at IS NULL OR julianday(last_seen_at) IS NOT NULL);
