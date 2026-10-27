-- The site search index (ADR "Site search is our own inverted index in
-- SQLite"). Both tables are derived from the published posts and rebuilt
-- from them, like the content index. Terms are words in lower case, cut to
-- their stem by khalid-search, so the query goes through the same rules.

-- One row per page search can find: where it is, what a result shows, and
-- how many words each field holds, which ranking weighs mentions against.
CREATE TABLE search_pages (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    path TEXT NOT NULL,
    href TEXT NOT NULL,
    title TEXT NOT NULL,
    -- The body as plain text, which a result's snippet is cut from.
    body TEXT NOT NULL,
    title_words INTEGER NOT NULL CHECK (title_words >= 0),
    heading_words INTEGER NOT NULL CHECK (heading_words >= 0),
    body_words INTEGER NOT NULL CHECK (body_words >= 0),
    UNIQUE (site_id, path)
) STRICT;

CREATE INDEX search_pages_by_locale ON search_pages (site_id, locale);

-- The inverted index: for each term, the pages that hold it and how often,
-- in the title, the headings and the body. A query reads only the rows of
-- its own terms, including every term that starts with a word being typed.
CREATE TABLE search_terms (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    term TEXT NOT NULL,
    page_id INTEGER NOT NULL REFERENCES search_pages (id) ON DELETE CASCADE,
    in_title INTEGER NOT NULL CHECK (in_title >= 0),
    in_headings INTEGER NOT NULL CHECK (in_headings >= 0),
    in_body INTEGER NOT NULL CHECK (in_body >= 0),
    PRIMARY KEY (site_id, term, page_id)
) STRICT, WITHOUT ROWID;

-- Removing a page removes its terms by cascade, which looks them up here.
CREATE INDEX search_terms_by_page ON search_terms (page_id);
