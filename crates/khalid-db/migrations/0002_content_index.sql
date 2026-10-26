-- The content index: everything here is derived from the files under
-- content/ and is rebuilt from them on demand (ADR 0001). Visitor tables come
-- in their own migration. Every table carries site_id, so one process can
-- serve several sites. Migrations are forward-only (RULES.md BL-DB-2): never
-- edit this file once it is on main; add a new one.
--
-- Paths are relative to the site folder, such as content/en/posts/hello.md.
-- Content hashes are the 32-byte SHA-256 from khalid-content.

CREATE TABLE authors (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    content_hash BLOB NOT NULL CHECK (length(content_hash) = 32),
    UNIQUE (site_id, locale, slug),
    UNIQUE (site_id, path)
) STRICT;

CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (site_id, locale, slug)
) STRICT;

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (site_id, locale, slug)
) STRICT;

-- Author and category are slugs, not foreign keys: a post may name an author
-- with no file yet, and the linter, not the database, explains that.
CREATE TABLE posts (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    collection TEXT NOT NULL,
    slug TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    description TEXT,
    date TEXT CHECK (date IS NULL OR date(date) = date),
    author TEXT,
    category TEXT,
    content_hash BLOB NOT NULL CHECK (length(content_hash) = 32),
    UNIQUE (site_id, path)
) STRICT;

-- A collection's index page lists its posts newest first.
CREATE INDEX posts_by_date ON posts (site_id, locale, collection, date DESC, slug);

CREATE TABLE post_tags (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    post_id INTEGER NOT NULL REFERENCES posts (id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, tag_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX post_tags_by_tag ON post_tags (tag_id, post_id);

CREATE TABLE pages (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    slug TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    description TEXT,
    content_hash BLOB NOT NULL CHECK (length(content_hash) = 32),
    UNIQUE (site_id, path)
) STRICT;

-- Entries of the other collections: glossary, case studies, changelog.
CREATE TABLE collection_entries (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    collection TEXT NOT NULL,
    slug TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    date TEXT CHECK (date IS NULL OR date(date) = date),
    content_hash BLOB NOT NULL CHECK (length(content_hash) = 32),
    UNIQUE (site_id, path)
) STRICT;

CREATE INDEX collection_entries_by_slug ON collection_entries (site_id, locale, collection, slug);

-- The language versions of one piece of content share a translation key.
CREATE TABLE translations (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    locale TEXT NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (site_id, key, locale),
    UNIQUE (site_id, path)
) STRICT, WITHOUT ROWID;

-- The link graph. target is the link as written; to_path is the content file
-- it resolves to, or NULL for a link that leaves the site or is broken.
CREATE TABLE links (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    from_path TEXT NOT NULL,
    line INTEGER NOT NULL CHECK (line > 0),
    target TEXT NOT NULL,
    to_path TEXT,
    anchor_text TEXT NOT NULL
) STRICT;

CREATE INDEX links_from ON links (site_id, from_path, line);
CREATE INDEX links_to ON links (site_id, to_path, from_path);

CREATE TABLE lint_results (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    rule TEXT NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('block', 'warn', 'note')),
    line INTEGER CHECK (line IS NULL OR line > 0),
    message TEXT NOT NULL
) STRICT;

CREATE INDEX lint_results_by_path ON lint_results (site_id, path, line);

-- Old addresses listed under `aliases` in front matter. Redirects people
-- create in the admin are visitor data and live in their own table.
CREATE TABLE alias_redirects (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    from_path TEXT NOT NULL,
    to_path TEXT NOT NULL,
    PRIMARY KEY (site_id, from_path)
) STRICT, WITHOUT ROWID;
