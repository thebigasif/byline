-- The sites a process serves. Every other table carries a site_id that points
-- here. Migrations are forward-only (RULES.md BL-DB-2): never edit this file
-- once it is on main; add a new one.

CREATE TABLE sites (
    id INTEGER PRIMARY KEY,
    handle TEXT NOT NULL UNIQUE
) STRICT;
