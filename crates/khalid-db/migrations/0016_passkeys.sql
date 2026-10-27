-- Staff passkeys (khalid-auth, ADR "staff accounts in khalid-auth"): the
-- authenticator's name for each, its public key as khalid-auth stores it
-- (one byte for the algorithm, then 65 or 32 bytes), and its signature
-- counter. Migrations are forward-only (RULES.md BL-DB-2).
CREATE TABLE passkeys (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential_id BLOB NOT NULL CHECK (length(credential_id) BETWEEN 1 AND 1023),
    public_key BLOB NOT NULL CHECK (length(public_key) IN (33, 66)),
    sign_count INTEGER NOT NULL CHECK (sign_count BETWEEN 0 AND 4294967295),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    last_used_at TEXT CHECK (last_used_at IS NULL OR julianday(last_used_at) IS NOT NULL),
    UNIQUE (site_id, credential_id)
) STRICT;

CREATE INDEX passkeys_by_user ON passkeys (user_id);
