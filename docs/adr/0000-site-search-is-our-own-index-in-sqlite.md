# 0000. Site search is our own inverted index in SQLite, stemmed in Rust

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5 work pulled forward, the search track (KH-WEB-2, KH-OPS-1).

## Context

Plan section 7 and Phase 5 ask for site search in server mode: built at publish time, ranked by title, headings and body, in each language, and a plain `GET` form that works with scripts off. Static mode cannot answer a query, so it shows a lock card instead. The database already holds what is computed from the files (ADR 0001, ADR 0010).

SQLite ships a full-text engine, FTS5. Its word rules can stem only English (the Porter rules); a German stemmer would have to be a C extension, which needs `unsafe`, forbidden here. FTS5 also stores its index in five hidden tables with no `site_id`, which the schema tests refuse, and its snippets show the stored text, which would be the stems, not what the writer wrote.

## Decision

- `khalid-search` turns text into terms: words split on letters and digits, lower case, then cut to their stem with the Snowball rules for English and German from `rust-stemmers`. Other languages match whole words. The reader's query goes through the same rules.
- The index is two ordinary tables in `data/site.db`: one row per searchable page with the text a snippet is cut from, and one row per term and page with how often the term appears in the title, the headings and the body. They are derived tables, rebuilt from the files like the rest.
- Ranking happens in Rust, over the rows the query's terms find: BM25 with the title counting more than a heading and a heading more than the body. Every term must appear, and the last one may be the start of a word, so results can show while a person types.
- Snippets are cut from the stored plain text, with each matching word marked as it was written.

## Consequences

- One new dependency, `rust-stemmers` (MIT or BSD-3-Clause), and no `unsafe`.
- Ranking can be tested as plain Rust, without a database.
- A query reads only the rows of its own terms, so it costs about the same on ten posts or ten thousand.
- A third stemmed language is one line in `Language` and its tests, as long as Snowball has rules for it.
