# Demo site

The fixture every benchmark, snapshot and end-to-end test uses from Phase 1 (task 0.5 in `docs/phases/phase-0.md`). It is small on purpose and written like a real blog, so a test that renders it shows what a reader would see.

| What | Where |
|---|---|
| Site settings, three categories | `site.toml` |
| Five posts, English and German | `content/en/posts/`, `content/de/posts/` |
| Two authors | `content/<language>/authors/` |
| One glossary term | `content/<language>/glossary/` |

Each English post and its German version share a `translation` key in the front matter, which is how hreflang pairs them. Every post links to at least one other post, and two use a `[[wikilink]]` to the glossary term.

The front matter fields follow `docs/plan.md` sections 4.3 and 5.4. Phase 1 fixes the schema, and a test there asserts this folder parses. If a field is renamed then, rename it here in the same pull request.
