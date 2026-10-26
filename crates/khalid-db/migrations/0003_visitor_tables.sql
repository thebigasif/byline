-- Visitor tables: what people create while the site runs. A rebuild never
-- touches them, and none of them holds a foreign key to a derived table, so
-- emptying the content index cannot delete a row here (ADR 0010). Content is
-- named by its path, such as content/en/posts/hello.md.
--
-- Times are UTC, written by Khalid's clock as RFC 3339 text. Tokens and keys
-- are stored as 32-byte SHA-256 hashes of 256 random bits, never in the clear
-- (RULES.md BL-SEC-3). Migrations are forward-only (BL-DB-2).

-- Redirects people create in the admin, bring in with an import, or that
-- Khalid adds when a slug changes. Aliases from front matter are derived and
-- live in alias_redirects.
CREATE TABLE redirects (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    from_path TEXT NOT NULL,
    to_url TEXT,
    status INTEGER NOT NULL CHECK (status IN (301, 302, 307, 410, 451)),
    origin TEXT NOT NULL CHECK (origin IN ('manual', 'import', 'slug_change')),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    -- 410 and 451 say the page is gone, so they have nowhere to send people.
    CHECK ((status IN (410, 451)) = (to_url IS NULL)),
    UNIQUE (site_id, from_path)
) STRICT;

CREATE TABLE not_found_log (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    hits INTEGER NOT NULL CHECK (hits > 0),
    first_seen TEXT NOT NULL CHECK (julianday(first_seen) IS NOT NULL),
    last_seen TEXT NOT NULL CHECK (julianday(last_seen) IS NOT NULL),
    PRIMARY KEY (site_id, path)
) STRICT, WITHOUT ROWID;

-- password_hash is an Argon2id string, or NULL for an account that signs in
-- with a passkey only.
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'editor', 'writer', 'contributor')),
    password_hash TEXT,
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    UNIQUE (site_id, email)
) STRICT;

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    expires_at TEXT NOT NULL CHECK (julianday(expires_at) > julianday(created_at))
) STRICT;

CREATE INDEX sessions_by_user ON sessions (user_id);

-- Integration keys have no user; staff tokens belong to one.
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    key_hash BLOB NOT NULL UNIQUE CHECK (length(key_hash) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    last_used_at TEXT CHECK (last_used_at IS NULL OR julianday(last_used_at) IS NOT NULL),
    revoked_at TEXT CHECK (revoked_at IS NULL OR julianday(revoked_at) IS NOT NULL)
) STRICT;

CREATE TABLE api_key_scopes (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    api_key_id INTEGER NOT NULL REFERENCES api_keys (id) ON DELETE CASCADE,
    scope TEXT NOT NULL,
    PRIMARY KEY (api_key_id, scope)
) STRICT, WITHOUT ROWID;

-- The signing secret has to be read back to sign each delivery, so it cannot
-- be hashed. It is never logged or returned by the API after creation.
CREATE TABLE webhooks (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    event TEXT NOT NULL,
    url TEXT NOT NULL,
    secret BLOB NOT NULL CHECK (length(secret) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    disabled_at TEXT CHECK (disabled_at IS NULL OR julianday(disabled_at) IS NOT NULL)
) STRICT;

CREATE INDEX webhooks_by_event ON webhooks (site_id, event);

CREATE TABLE webhook_deliveries (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    webhook_id INTEGER NOT NULL REFERENCES webhooks (id) ON DELETE CASCADE,
    payload TEXT NOT NULL CHECK (json_valid(payload)),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_status INTEGER,
    next_attempt_at TEXT CHECK (next_attempt_at IS NULL OR julianday(next_attempt_at) IS NOT NULL),
    delivered_at TEXT CHECK (delivered_at IS NULL OR julianday(delivered_at) IS NOT NULL),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL)
) STRICT;

-- The retry worker reads the deliveries that are due, oldest first.
CREATE INDEX webhook_deliveries_due ON webhook_deliveries (site_id, next_attempt_at)
    WHERE delivered_at IS NULL;

CREATE TABLE form_submissions (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    form TEXT NOT NULL,
    page_path TEXT,
    fields TEXT NOT NULL CHECK (json_valid(fields)),
    status TEXT NOT NULL CHECK (status IN ('received', 'spam')),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL)
) STRICT;

CREATE INDEX form_submissions_by_form ON form_submissions (site_id, form, created_at);

CREATE TABLE subscribers (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'confirmed', 'unsubscribed')),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    confirmed_at TEXT CHECK (confirmed_at IS NULL OR julianday(confirmed_at) IS NOT NULL),
    UNIQUE (site_id, email)
) STRICT;

CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_path TEXT NOT NULL,
    parent_id INTEGER REFERENCES comments (id) ON DELETE CASCADE,
    author_name TEXT NOT NULL,
    author_email TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'spam', 'trash')),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL)
) STRICT;

CREATE INDEX comments_by_post ON comments (site_id, post_path, status, created_at);

-- Counted on the server with no cookies: one row per day, page and referrer
-- group, such as search, social or ai.
CREATE TABLE analytics_counts (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    day TEXT NOT NULL CHECK (date(day) = day),
    path TEXT NOT NULL,
    referrer_group TEXT NOT NULL,
    views INTEGER NOT NULL CHECK (views > 0),
    PRIMARY KEY (site_id, day, path, referrer_group)
) STRICT, WITHOUT ROWID;
