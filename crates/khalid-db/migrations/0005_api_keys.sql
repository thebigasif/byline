-- API keys in the shape khalid-api gives them (ADR 0011): every key is
-- looked up by its public identity, 16 hex digits, and stores a hash of its
-- secret. Content keys store the SHA-256 as 64 hex digits; integration keys
-- and staff tokens store an Argon2id hash in PHC form. Nothing has written
-- to api_keys yet, so it is replaced rather than altered. Migrations are
-- forward-only (RULES.md BL-DB-2).
DROP TABLE api_key_scopes;
DROP TABLE api_keys;

-- Times are UTC, written by Khalid's clock as RFC 3339 text. A revoked key
-- keeps its row, so its identity is never handed out again.
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users (id) ON DELETE CASCADE,
    key_id TEXT NOT NULL UNIQUE CHECK (length(key_id) = 16 AND key_id NOT GLOB '*[^0-9a-f]*'),
    kind TEXT NOT NULL CHECK (kind IN ('content', 'integration', 'staff')),
    -- A staff token's role; integration keys list scopes in api_key_scopes.
    role TEXT CHECK (role IN ('owner', 'admin', 'editor', 'writer', 'contributor')),
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    last_used_at TEXT CHECK (last_used_at IS NULL OR julianday(last_used_at) IS NOT NULL),
    revoked_at TEXT CHECK (revoked_at IS NULL OR julianday(revoked_at) IS NOT NULL),
    CHECK ((kind = 'staff') = (role IS NOT NULL)),
    CHECK (
        (kind = 'content' AND length(key_hash) = 64 AND key_hash NOT GLOB '*[^0-9a-f]*')
        OR (kind != 'content' AND key_hash GLOB '$argon2id$*')
    )
) STRICT;

CREATE TABLE api_key_scopes (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    api_key_id INTEGER NOT NULL REFERENCES api_keys (id) ON DELETE CASCADE,
    scope TEXT NOT NULL,
    PRIMARY KEY (api_key_id, scope)
) STRICT, WITHOUT ROWID;
