-- Keys a site makes for itself and keeps, such as the one that signs its
-- preview links (CTO answers 2026-10-27, section 4). Each is 256 random
-- bits, made once on first use, and never logged or sent anywhere. A lost
-- database makes a new key, so links signed with the old one stop working.
-- Migrations are forward-only (RULES.md BL-DB-2).
CREATE TABLE site_keys (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('preview')),
    secret BLOB NOT NULL CHECK (length(secret) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    PRIMARY KEY (site_id, purpose)
) STRICT, WITHOUT ROWID;
