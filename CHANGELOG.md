# Changelog

What changed for people who use Khalid, one line per change, newest first. Grouped by Added, Changed, Fixed and Removed. Every pull request that changes behaviour adds its line under Unreleased (`docs/standards/git.md`).

## Unreleased

### Added

- `khalid --site <folder> db rebuild` fills the site's index from its posts again. Comments, subscribers and other visitor data are kept.
- `khalid --site <folder> db migrate` makes the site's database at `data/site.db`, or brings it up to date.
- `khalid --help` lists every planned command: init, serve, new, lint, publish, theme, import, export, mcp, doctor, backup, audit, update, pull and keys.
- Each command explains in three plain sentences that it is not built yet, and exits with code 3.
- `--json` on every command prints the same message as JSON, for scripts and agents.
- A mistyped command gets a plain explanation and exit code 2, instead of a wall of usage text.
- `khalid --site <folder>` checks the site's settings and reads every content file: posts, pages, authors, glossary terms, case studies, changelog entries and snippets, as plain files or as folders with their images. A mistake is explained by file, line and field (KH-CON-1, KH-CON-2, KH-CON-3, KH-CON-4).
- Posts that share a `translation` key link to each other with hreflang, plus `x-default` for readers in any other language. Every version carries the same complete list (KH-CON-8).
- The home page carries the verification codes for Google, Bing, Yandex, Baidu and Pinterest, and a verification file can be served from the root of the address (KH-SEO-7).
- The home page carries the verification codes for Google, Bing, Yandex, Baidu and Pinterest, and a verification file can be served from the root of the address (KH-SEO-7).
- Posts render to HTML with headings you can link to, coloured code, tables, footnotes, tip boxes (`:::callout`) and captioned images (`:::figure`), and every block writes back to markdown unchanged (KH-CON-5).
- Every page gets Open Graph and Twitter card tags, so a shared link shows the right title, summary and image. A site sets one default share image and a post can set its own (KH-SEO-2).
- Every page gets a title, a description, a canonical address and robots directives from templates per page type, with a per-page override. Search results, date archives and thin tag or author archives stay out of search results by default (KH-SEO-1, KH-SEO-9).
- Release builds for Linux (x86_64 and ARM), macOS and Windows, with checksums, from a version tag.
