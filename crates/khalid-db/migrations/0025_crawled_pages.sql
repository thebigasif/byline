-- Pages a bot fetched from an address on its company's list (GEO C2 to
-- C4): one row per page and bot, with the first and last fetch answered
-- with the page. Written with the visitor log, only while [visitor_log] is
-- on, and kept when its rows are erased. No address and no visitor code.
CREATE TABLE crawled_pages (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    crawler TEXT NOT NULL,
    first_at TEXT NOT NULL CHECK (julianday(first_at) IS NOT NULL),
    last_at TEXT NOT NULL CHECK (julianday(last_at) IS NOT NULL),
    CHECK (first_at <= last_at),
    PRIMARY KEY (site_id, path, crawler)
) STRICT, WITHOUT ROWID;
