-- Newsletter sends (tasks 5.8 and 5.12). One row per email that went out
-- to a newsletter, under a key such as post:en/hello, so the same post is
-- never sent twice. Totals only: who got a copy is never stored.
CREATE TABLE newsletter_sends (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    send_key TEXT NOT NULL CHECK (length(send_key) > 0),
    list TEXT NOT NULL CHECK (length(list) > 0),
    started_at TEXT NOT NULL CHECK (julianday(started_at) IS NOT NULL),
    finished_at TEXT CHECK (finished_at IS NULL OR julianday(finished_at) IS NOT NULL),
    sent INTEGER NOT NULL DEFAULT 0 CHECK (sent >= 0),
    failed INTEGER NOT NULL DEFAULT 0 CHECK (failed >= 0),
    unsubscribed INTEGER NOT NULL DEFAULT 0 CHECK (unsubscribed >= 0),
    UNIQUE (site_id, send_key)
) STRICT;

-- The email an unsubscribe link came in, so a click counts against it.
-- NULL for the link in a confirmation email.
ALTER TABLE subscriber_tokens ADD COLUMN send_id INTEGER
    REFERENCES newsletter_sends (id) ON DELETE SET NULL;
