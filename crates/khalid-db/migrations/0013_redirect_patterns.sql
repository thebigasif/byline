-- A stored redirect matches one path exactly, or every path its regular
-- expression matches (KH-SEO-8). For a regex rule, from_path holds the
-- pattern as written and to_url may use its groups as $1. Rows already
-- stored match exactly. Migrations are forward-only (RULES.md BL-DB-2).
ALTER TABLE redirects ADD COLUMN match_kind TEXT NOT NULL DEFAULT 'exact'
    CHECK (match_kind IN ('exact', 'regex'));
