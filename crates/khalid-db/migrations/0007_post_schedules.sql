-- Each post's schedule, kept by the publish door (CTO answers 2026-10-27,
-- section 4, and the ADR "schedules live in the database"). A row is named
-- by the post's identity, <language>.<slug>, as the Admin API names it, so
-- it survives a rebuild of the index. Times are UTC, written by Khalid's
-- clock as RFC 3339 text to the second, so they sort as text. A row with
-- neither time is removed instead. Migrations are forward-only (BL-DB-2).
CREATE TABLE post_schedules (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_id TEXT NOT NULL CHECK (length(post_id) > 0),
    publish_at TEXT CHECK (publish_at IS NULL OR julianday(publish_at) IS NOT NULL),
    unpublish_at TEXT CHECK (unpublish_at IS NULL OR julianday(unpublish_at) IS NOT NULL),
    CHECK (publish_at IS NOT NULL OR unpublish_at IS NOT NULL),
    PRIMARY KEY (site_id, post_id)
) STRICT, WITHOUT ROWID;
