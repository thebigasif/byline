-- Each post's earlier versions, kept by the publish door (plan 4.6,
-- KH-EDT-2): the title and text an edit replaced, with when and by whom
-- they were saved. A row is named by the post's identity,
-- <language>.<slug>, as the Admin API names it, so a rebuild of the index
-- keeps it. kept is a counter that orders a post's versions, the newest
-- highest; only the 100 newest of a post are kept. Times are UTC, written
-- as RFC 3339 text to the second. Migrations are forward-only (BL-DB-2).
CREATE TABLE post_versions (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_id TEXT NOT NULL CHECK (length(post_id) > 0),
    kept INTEGER NOT NULL CHECK (kept > 0),
    saved_at TEXT NOT NULL CHECK (julianday(saved_at) IS NOT NULL),
    saved_by TEXT,
    title TEXT NOT NULL,
    markdown TEXT NOT NULL,
    PRIMARY KEY (site_id, post_id, kept)
) STRICT, WITHOUT ROWID;
