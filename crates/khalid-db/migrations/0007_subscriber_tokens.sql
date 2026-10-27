-- Double opt-in and one-click unsubscribe (task 5.5). Visitor tables, with
-- tokens stored as SHA-256 hashes only (RULES.md BL-SEC-3).

-- When a subscriber left; NULL for rows from before this migration.
ALTER TABLE subscribers ADD COLUMN unsubscribed_at TEXT
    CHECK (unsubscribed_at IS NULL OR julianday(unsubscribed_at) IS NOT NULL);

-- Which newsletters each subscriber gets, by name. Unsubscribing deletes them.
CREATE TABLE subscriber_lists (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    subscriber_id INTEGER NOT NULL REFERENCES subscribers (id) ON DELETE CASCADE,
    list TEXT NOT NULL CHECK (length(list) > 0),
    joined_at TEXT NOT NULL CHECK (julianday(joined_at) IS NOT NULL),
    PRIMARY KEY (subscriber_id, list)
) STRICT, WITHOUT ROWID;

CREATE INDEX subscriber_lists_by_list ON subscriber_lists (site_id, list);

-- The links in emails. A confirmation link expires and works once. An
-- unsubscribe link never expires, and clicking it again changes nothing.
CREATE TABLE subscriber_tokens (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    subscriber_id INTEGER NOT NULL REFERENCES subscribers (id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('confirm', 'unsubscribe')),
    token_hash BLOB NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL),
    expires_at TEXT CHECK (expires_at IS NULL OR julianday(expires_at) > julianday(created_at)),
    used_at TEXT CHECK (used_at IS NULL OR julianday(used_at) IS NOT NULL),
    CHECK ((purpose = 'confirm') = (expires_at IS NOT NULL))
) STRICT;

-- A sign-up looks for a confirmation sent a moment ago.
CREATE INDEX subscriber_tokens_by_subscriber
    ON subscriber_tokens (subscriber_id, purpose, created_at);
