-- The audit log (KH-EDT-6): one row per change made through the Admin API,
-- with who made it, what it did, the route and the address. Visitor data:
-- a rebuild keeps it. The newest 100,000 rows per site are kept.
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    at TEXT NOT NULL CHECK (julianday(at) IS NOT NULL),
    by_key TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('added', 'changed', 'removed')),
    route TEXT NOT NULL,
    path TEXT NOT NULL
) STRICT;

CREATE INDEX audit_log_by_site ON audit_log (site_id, id);
