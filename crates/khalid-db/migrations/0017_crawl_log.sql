-- The crawl log (KH-GRO-4): one row per day, address and bot, named as its
-- user agent claimed. Visitor data, kept 90 days.
CREATE TABLE crawl_log (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    day TEXT NOT NULL CHECK (date(day) = day),
    path TEXT NOT NULL,
    crawler TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('search', 'ai', 'seo', 'social', 'other')),
    hits INTEGER NOT NULL CHECK (hits > 0),
    last_seen TEXT NOT NULL CHECK (julianday(last_seen) IS NOT NULL),
    last_status INTEGER NOT NULL CHECK (last_status BETWEEN 100 AND 599),
    PRIMARY KEY (site_id, day, path, crawler)
) STRICT, WITHOUT ROWID;
