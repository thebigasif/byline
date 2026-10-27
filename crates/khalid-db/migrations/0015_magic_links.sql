-- Sign-in links sent by email (khalid-auth, ADR "staff accounts in
-- khalid-auth"), stored as the SHA-256 of their token. A link works once,
-- before it expires. Migrations are forward-only (RULES.md BL-DB-2).
CREATE TABLE magic_links (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    expires_at TEXT NOT NULL CHECK (julianday(expires_at) > julianday(created_at)),
    used_at TEXT CHECK (used_at IS NULL OR julianday(used_at) IS NOT NULL)
) STRICT;

CREATE INDEX magic_links_by_user ON magic_links (user_id);
