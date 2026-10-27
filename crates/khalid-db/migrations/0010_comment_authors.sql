-- Comments (task 5.4). A reader gives a name and may give an email, which is
-- never shown, so author_email may be empty (Phase 5 answers, 2026-10-27).
-- Staff answer through the Admin API with no email at all, so each comment
-- says who wrote it. SQLite cannot change a column, so the table is made
-- again and its rows copied. A rebuild never touches it (ADR 0010).
CREATE TABLE comments_new (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_path TEXT NOT NULL,
    -- Threads are one level deep, so a parent never has a parent itself.
    parent_id INTEGER REFERENCES comments_new (id) ON DELETE CASCADE,
    author_kind TEXT NOT NULL CHECK (author_kind IN ('reader', 'staff')),
    author_name TEXT NOT NULL CHECK (author_name <> ''),
    author_email TEXT CHECK (author_email IS NULL OR author_kind = 'reader'),
    body TEXT NOT NULL CHECK (body <> ''),
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'spam', 'trash')),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL)
) STRICT;

INSERT INTO comments_new
    (id, site_id, post_path, parent_id, author_kind, author_name, author_email, body, status, created_at)
SELECT id, site_id, post_path, parent_id, 'reader', author_name, NULLIF(author_email, ''), body, status, created_at
FROM comments;

DROP TABLE comments;

-- Renaming also points parent_id at the renamed table.
ALTER TABLE comments_new RENAME TO comments;

CREATE INDEX comments_by_post ON comments (site_id, post_path, status, created_at);
CREATE INDEX comments_by_parent ON comments (parent_id);
