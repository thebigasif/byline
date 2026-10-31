-- Staff comments on drafts (KH-EDT-4): threaded one level deep, tied to a
-- block of the body, resolved when dealt with. Named by the post's
-- identity, as post_schedules is, so a rebuild keeps them. Visitor data.
CREATE TABLE draft_comments (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_id TEXT NOT NULL CHECK (post_id <> ''),
    -- A reply names its thread's first comment, which has no parent.
    parent_id INTEGER REFERENCES draft_comments (id) ON DELETE CASCADE,
    block INTEGER CHECK (block IS NULL OR block > 0),
    author TEXT NOT NULL CHECK (author <> ''),
    body TEXT NOT NULL CHECK (trim(body) <> '' AND length(body) <= 5000),
    -- Only a thread's first comment is ever resolved.
    resolved INTEGER NOT NULL DEFAULT 0
        CHECK (resolved IN (0, 1) AND (resolved = 0 OR parent_id IS NULL)),
    created_at TEXT NOT NULL CHECK (julianday(created_at) IS NOT NULL)
) STRICT;

CREATE INDEX draft_comments_by_post ON draft_comments (site_id, post_id, id);
