-- Staff passkeys are checked by webauthn-rs (ADR "passkeys are checked by
-- webauthn-rs"): each row keeps the passkey as webauthn-rs saves it, in
-- JSON, with its counter inside. The rows of 0016 held Khalid's own key
-- form, which webauthn-rs cannot read, and Khalid had no release, so they
-- go and staff add their passkeys again. Migrations are forward-only
-- (RULES.md BL-DB-2).
DROP TABLE passkeys;

CREATE TABLE passkeys (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    credential_id BLOB NOT NULL CHECK (length(credential_id) BETWEEN 1 AND 1023),
    credential TEXT NOT NULL CHECK (json_valid(credential)),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    last_used_at TEXT CHECK (last_used_at IS NULL OR julianday(last_used_at) IS NOT NULL),
    UNIQUE (site_id, credential_id)
) STRICT;

CREATE INDEX passkeys_by_user ON passkeys (user_id);
