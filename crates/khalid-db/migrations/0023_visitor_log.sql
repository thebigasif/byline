-- The visitor log (GEO C1): one row per visit, written only when
-- [visitor_log] is on in site.toml. Visitor data, kept 7 to 14 days.
-- `address` is shortened (IPv4 /24, IPv6 /48) and `visitor_code` is a
-- hash under a key that lives in memory one day; docs/design/visitor-log.md.
-- A person's row has a referrer group; a bot's row has its name and check.
CREATE TABLE visitor_log (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    at TEXT NOT NULL CHECK (julianday(at) IS NOT NULL),
    path TEXT NOT NULL,
    status INTEGER NOT NULL CHECK (status BETWEEN 100 AND 599),
    address TEXT NOT NULL,
    visitor_code TEXT NOT NULL CHECK (length(visitor_code) = 16),
    referrer_group TEXT,
    crawler TEXT,
    bot_check TEXT CHECK (bot_check IN ('listed', 'not-listed', 'no-list')),
    CHECK ((crawler IS NULL) = (bot_check IS NULL)),
    CHECK ((crawler IS NULL) = (referrer_group IS NOT NULL))
) STRICT;

CREATE INDEX visitor_log_by_time ON visitor_log (site_id, at);
