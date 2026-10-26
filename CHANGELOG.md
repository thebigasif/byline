# Changelog

What changed for people who use Khalid, one line per change, newest first. Grouped by Added, Changed, Fixed and Removed. Every pull request that changes behaviour adds its line under Unreleased (`docs/standards/git.md`).

## Unreleased

### Added

- `khalid --help` lists every planned command: init, serve, new, lint, publish, theme, import, export, mcp, doctor, backup, audit, update, pull and keys.
- Each command explains in three plain sentences that it is not built yet, and exits with code 3.
- `--json` on every command prints the same message as JSON, for scripts and agents.
- A mistyped command gets a plain explanation and exit code 2, instead of a wall of usage text.
- `khalid --site <folder>` checks the site's settings and reads every content file: posts, pages, authors, glossary terms, case studies, changelog entries and snippets, as plain files or as folders with their images. A mistake is explained by file, line and field (KH-CON-1, KH-CON-2, KH-CON-3, KH-CON-4).
- How-to blocks write their own schema, with steps, tools, supplies, time and cost (KH-SEO-3).
- FAQ blocks write their own schema, so the questions can show in search results (KH-SEO-3).
- A local business can list each location in schema markup, with its business type, address, opening hours, phone, price range and map (KH-SEO-3).
- Glossary terms are marked up as defined terms of the glossary, in each language (KH-SEO-3).
- A post's author is described in full in schema markup (job title, bio, profiles, topics), and the post names the things it is about and mentions (KH-SEO-3).
- Every post's schema is an article of the right type, with its headline, images, dates, word count, section and author (KH-SEO-3).
- Every page's schema names the page itself: a web page, a collection, a profile or search results, with its main image and the parts a voice assistant may read aloud (KH-SEO-3).
- Every page carries its breadcrumb trail in schema markup, from the home page of its language (KH-SEO-3).
- Every page names its publisher (a company or a person, with logo and profiles) and the site with its search box in schema markup (KH-SEO-3).
- Schema markup is written as one JSON-LD graph per page, in a script tag that no text in a post can break (KH-SEO-3).
- A site-wide link policy: external links can get `nofollow`, `sponsored` or `ugc` and open in a new tab, and links to affiliate domains are always `sponsored` (KH-SEO-10).
- A redirect can match many old addresses with a pattern, such as every `/2019/05/<slug>/` to `/<slug>/` (KH-SEO-8).
- Redirects: moved for good (301), for now (302 or 307), gone (410) or blocked for a legal reason (451). A chain of redirects answers in one hop, a loop is refused, and a debug view shows what each rule answers (KH-SEO-8).
- JSON feeds at `feed.json`, for apps and scripts that read JSON (KH-SEO-6).
- Atom feeds at `atom.xml`, for readers that prefer Atom (KH-SEO-6).
- RSS feeds at `feed.xml`, with the full post for readers that show it (KH-SEO-6).
- Feeds per language and per category hold the newest posts, in full or as excerpts, with optional text before and after each post and a line linking the original (KH-SEO-6).
- `llms.txt` gives language models a plain list of the blog's posts and pages with their summaries (KH-SEO-7).
- `robots.txt` lets every crawler in by default, names the sitemap, and can block or allow each crawler by name, such as the AI crawlers, or keep a folder private from all of them (KH-SEO-7).
- An HTML sitemap for readers lists every published page in a language, by section and title (KH-SEO-5).
- Sitemaps come as one file per section (posts, pages, archives) and language, split at 50,000 addresses, each listed in the index (KH-SEO-5).
- One sitemap index at `sitemap.xml` in the blog's folder lists every sitemap file (KH-SEO-5).
- A news sitemap lists news articles from the last two days, for Google News (KH-SEO-5).
- The sitemap lists each page's images and videos, so they can appear in image and video search (KH-SEO-5).
- An XML sitemap lists every published page search engines may index, with its last change and its language versions. Drafts, scheduled posts, thin archives and pages marked to stay out are left out (KH-SEO-5).
- Posts that share a `translation` key link to each other with hreflang, plus `x-default` for readers in any other language. Every version carries the same complete list (KH-CON-8).
- The home page carries the verification codes for Google, Bing, Yandex, Baidu and Pinterest, and a verification file can be served from the root of the address (KH-SEO-7).
- Every page gets Open Graph and Twitter card tags, so a shared link shows the right title, summary and image. A site sets one default share image and a post can set its own (KH-SEO-2).
- Every page gets a title, a description, a canonical address and robots directives from templates per page type, with a per-page override. Search results, date archives and thin tag or author archives stay out of search results by default (KH-SEO-1, KH-SEO-9).
- Release builds for Linux (x86_64 and ARM), macOS and Windows, with checksums, from a version tag.
