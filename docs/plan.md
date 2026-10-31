> This is the CMS copy of the build plan, written in the private SerpDad app repository on 2026-10-26. Where it says "this repo" in sections 3 and 8, it means that SerpDad repository. Everything else applies here.

# Plan: the open-source blog CMS by SerpDad

Written 2026-10-26 by Claude at the founder's request, from the founder's answers of the same day. **This is a plan only.** Nothing is built until the founder says yes to section 12.

The product is named Khalid, chosen by the founder on 2026-10-26 (see `docs/decisions/name.md`). Earlier drafts used Byline as a working name.

---

## 0. Summary in plain words

- **The one-line idea.** "Get an Ahrefs- or Semrush-quality blog in one click." A free, open-source blog engine any business can add to its website, whatever the site is built with.
- **What makes it different.** It copies the main site's colours, fonts and logo, so the blog looks native. It ships the best SEO defaults built in, with schema markup for every content type and a built-in site audit that passes the Ahrefs and Semrush checks, so there is nothing to install. It refuses to publish a post that breaks basic SEO hygiene, and every refusal teaches why. It is run from the command line, by people or by agents such as Claude Code. There is a web editor for writing, but no drag-and-drop page builder. Every public page is semantic HTML, responsive on every screen, and reads fully with JavaScript blocked.
- **Why SerpDad builds it.** Four reasons. It is a free tool that brings people to SerpDad. It teaches SEO basics, so users grow into SerpDad customers. It is the publish target at the end of the SerpDad content journey: "Publish with khalid by SerpDad". And SerpDad's own blog runs on it.
- **Shape.** One Rust program that is the server, the command line, the agent interface and the web admin at once. Every post is a plain text file. One small database per site holds everything derived from those files and everything visitors create. Two modes: server mode has everything; static mode exports plain HTML and shows the server features locked, with a plain explanation. Everything the admin can do, a public API can do too, so any front end can read the posts and any script or agent can write them.
- **Licence.** AGPL-3.0, so nobody can take it closed. Plus a trademark rule on the name and a short contributor agreement, so SerpDad can keep using every contribution in the paid product.
- **Size.** Eight phases. About 18 weeks of agent work in sequence, or 12 to 13 weeks with two or three agents running at once. Its own GitHub repository, not this one.

---

## 1. The founder's decisions (2026-10-26)

| # | Question | Decision | What it means for the build |
|---|---|---|---|
| 1 | How does the blog appear on the site? | A proxy rule only when the site needs one. A site that already has a blog at its own address needs none. | The server serves one site at whatever address it is given. `khalid init` asks where the blog will live and writes the proxy rule for that stack only when the blog sits under a folder of a site built with something else. |
| 2 | Who may use it? | Anyone, free, open source, with a step-by-step guide in the GitHub repo. | Public repo. A README that takes a developer from zero to a first post in ten minutes. A docs folder. Releases with prebuilt binaries. |
| 3 | Web editor? | Yes. | A writing editor built into the program. It writes the same text files the command line writes. No page builder. |
| 4 | One site or many? | One process per site. People with many sites ask Claude Code to install it for each. The dashboard has a super-admin panel to add more domains. | The program can serve many sites, but the default install is one. Adding a domain in the super-admin panel adds a site to the same process. See D-2 in section 12. |
| 5 | Lint: block or warn? | Block when a post has no date, no author or no internal link. Warn when an anchor text used before now points at a different page. Warn for everything else. Each message teaches. | Section 6 lists every rule with its severity and message. Severities live in a `lint.toml` people can tune. |
| 6 | Migration | One import command per source. | `khalid import wordpress`, `khalid import ghost`, and so on. Each has a dry run and a report. |
| 7 | Server features and static sites | Everything exists. Static users see server features locked, with an explanation. | Section 7 lists what works in each mode and the lock text. |
| 8 | Name and marketing | A nice name, "by SerpDad". Marketed from the free tools page and inside the SerpDad dashboard. "Publish with <name> by SerpDad" at the end of the content journey. | Section 8. The CMS becomes a fifth draft target next to WordPress, Webflow, Shopify and Google Docs. |
| 9 | Licence | AGPL-3.0. Free to use, nobody may resell it. | Section 9 explains what AGPL does and does not stop, and adds the two things that close the gap. |
| 10 | Languages | Multilingual and hreflang at launch. | Section 5.4. Language is part of the content model from day one. |
| 11 | Markup | Semantic HTML by default. | Default templates use the right element for every part of the page. Sections 4.5, 6 and 11. |
| 12 | JavaScript | Every page loads with JavaScript blocked. | The public blog never needs JavaScript. Links, forms, search, comments and the newsletter work as plain HTML, and JavaScript only adds polish. The admin is the one exception. |
| 13 | Devices | Responsive on every device by default. | Mobile-first templates, fluid type and spacing, no horizontal scroll from 320 px up. Section 4.10. |
| 14 | Front-end quality | All front-end best practices by default. | A named checklist in the repo: WCAG 2.2 AA, Core Web Vitals, valid HTML, zero layout shift, no third-party scripts. Section 4.10 and the measures in section 11. |
| 15 | Feature coverage | Match Rank Math, and the on-site parts of Ahrefs and Semrush. Schema markup comes with the CMS. | Appendix A checks every feature. Sections 5.9 to 5.11 add what was missing. |
| 16 | Open-source parity | Adopt the Appendix B "add to v1" list: a public API, routing, editor cards, workflow, media, forms and growth features. D-11, decided 2026-10-26. | Folded into sections 4 to 11. Phase 1 gains a week, Phase 4 gains two. |
| 17 | Extra collections | A glossary, case studies and a changelog as fixed, typed collections. D-12, decided 2026-10-26. | Section 5.13. Not a general content-type builder. |

---

## 2. What it is, and what it is not

| It is | It is not |
|---|---|
| A blog engine you add to an existing website | A tool to build the whole website |
| One program, one file to install | A stack of PHP, MySQL and a web server |
| A writing editor | A drag-and-drop page builder |
| SEO done by default and checked before publish | A checklist plugin you have to read |
| Semantic HTML that works with JavaScript off | A pile of `div`s that stays blank until scripts run |
| Responsive on every screen by default | A desktop layout squeezed onto a phone |
| Run by people and by agents from the command line | Click-only |
| Neutral and offline: no AI inside, no calls home | A SerpDad client in disguise |
| Free and open source under AGPL | Something a host can fork and sell |
| A public API any front end can read and any script can write | A closed box behind one theme |

### Four journeys

**A developer adds a blog to a company site.** They run one install line, then `khalid init`. It asks four things: the site's address, where the blog should live, the languages, and server or static. It fetches the site, pulls colours, fonts and logo into a theme file, and shows a preview. If the blog sits under a folder of a Next.js or Webflow site, it prints the proxy rule for that stack. Ten minutes later the blog is live and looks like the site.

**A developer who wants their own front end.** They keep their site as it is, point the Astro or Next.js starter kit at the Content API, and render the posts inside their own app. Schema, sitemaps, feeds and the linter still come from the CMS. Writers still use the editor, or they write in git and the blog pulls on push.

**A marketer writes a post.** They open the web editor, write, add images, and press Publish. The linter stops them: "This post links to none of your other posts. Internal links are how readers and Google find the rest of your site. Add one link to a related post." It suggests three. They add one and publish. The sitemap, feeds, schema and IndexNow ping all happen without them.

**A SerpDad user finishes the content journey.** The article is approved and written. Next to "Send to WordPress" there is "Publish with khalid by SerpDad". They connect their blog once with its address and a token. From then on each finished article lands in the blog as a draft, already linted, ready to publish.

---

## 3. How it fits SerpDad

- **Its own repository.** The SerpDad app is private. AGPL code cannot sit inside it. The CMS gets its own GitHub repository under the SerpDad organisation.
- **Shared Rust.** The CMS and the SerpDad backend both need an HTML fetcher, a link extractor and text checks. Today those live in private crates: `fetch` has `crawl` and `links`, and `text` has `article_metrics`, `banned_phrases`, `no_dashes` and `score_text`. The plan is to put the lint rules and the text checks in the public CMS repo as a crate, and have the SerpDad backend depend on that crate. Public rules are also a marketing asset: the rules SerpDad uses are the rules anyone can read.
- **Licensing of shared code.** SerpDad owns the copyright, so it may use its own code under any licence in the paid product. Outside contributions arrive under AGPL. A short contributor licence agreement, signed once by each contributor through a bot on the pull request, lets SerpDad use those too. Without it, every outside fix to the lint rules would be off limits for the SerpDad backend.
- **A fifth draft target.** SerpDad already stages drafts to WordPress, Webflow, Shopify and Google Docs through one `DraftTarget` contract. It sends title, HTML body, slug, excerpt and draft status. The CMS's Admin API, section 5.12, takes that shape plus markdown, and SerpDad is one client of it. Connecting it works like WordPress: site address plus a token made in the CMS admin. The Rust `integrations` crate gets a `khalid` module beside `wordpress` and `webflow`.
- **Neutral by default.** The open-source build never calls SerpDad or any AI. "Connect to SerpDad" is a settings page that is off until the user turns it on. This keeps the project trustworthy to people who will never pay, and they are the ones who star repos and write about them.
- **Customer zero.** serpdad.com gets its blog on the CMS in Phase 7. It is the proof, the demo and the dogfood.

---

## 4. Architecture in plain words

### 4.1 One program

One Rust binary. Run with no arguments it prints help. Its roles:

| Command | What it does |
|---|---|
| `khalid init` | Set up a site: address, mount, languages, mode. Pull the theme. Write the proxy rule if needed. |
| `khalid serve` | Run the blog and the admin. HTTPS by itself when given a domain. |
| `khalid new "Title"` | Create a post file with correct front matter. |
| `khalid lint [path]` | Check one post or the whole site. Exit code 1 on a block, 0 otherwise. `--json` for agents. |
| `khalid publish <slug>` | Lint, then flip a draft to published. Refuses on a block. |
| `khalid theme sync <url>` | Pull colours, fonts and logo again. Shows a diff. Manual edits win. |
| `khalid import <source>` | Bring in a blog from WordPress, Ghost, Medium, Substack or a folder of markdown. |
| `khalid export` | Static mode: write plain HTML to `public/`. |
| `khalid mcp` | Speak the Model Context Protocol on stdin and stdout, so Claude Code and other agents can do everything above. |
| `khalid doctor` | Check the install: DNS, HTTPS, proxy rule, sitemap reachable, robots, hreflang pairs complete, HTML valid, every page type whole with scripts stripped, viewport and image sizes set. |
| `khalid backup` | One archive of the site folder. |
| `khalid audit` | Crawl the site the way Ahrefs and Semrush do and score it. Section 5.10. |
| `khalid update` | Update the binary, with `--rollback` to the previous version. |
| `khalid theme check` | Validate a template override before it goes live, the way Ghost's gscan does. |
| `khalid pull` | Git mode: fetch the content repository and rebuild. A push webhook runs it too. |
| `khalid keys` | Create and revoke API keys for integrations. |

Every command has `--json`. Every error says what happened, why it matters and what to do.

### 4.2 Content is files, the database is for the rest

This is the point the founder asked about. In plain words:

- **Every post is a text file.** Markdown with a short header for title, date, author, language and such. It sits in a folder on disk. You can open it in any editor, search it, diff it, keep it in git, and let Claude Code write it.
- **The web editor edits those same files.** When a writer saves in the browser, the file on disk changes. Nothing is hidden in a database that only the editor can read.
- **The database holds what is computed or what visitors create.** The link graph between posts, lint results, the search index, redirects, the 404 log, newsletter subscribers, comments, analytics counts, scheduled jobs and user accounts. It is one SQLite file per site. Delete it and `khalid serve` rebuilds everything except visitor data from the files.

Why this and not "everything in a database" like WordPress: migration is a folder of files anyone can read, a backup is a copy, history is git, agents work with files natively, and the blog survives the CMS. Why not "only files" like a static site generator: the editor, search, comments, newsletter, scheduled publish and analytics all need somewhere to write.

### 4.3 A site on disk

```
my-blog/
  site.toml              address, mount path, languages, mode, policies
  theme/
    tokens.toml          colours, fonts, logo, radius, spacing, pulled from the main site
    templates/           only the files you override; defaults ship inside the binary
  routes.toml            collections, channels, permalink patterns, taxonomy prefixes
  content/
    en/
      posts/2026-10-26-why-internal-links-matter.md
      posts/choosing-a-crm/index.md        a post folder, with its images beside it
      posts/choosing-a-crm/hero.jpg
      pages/about.md
      authors/asif.md
      glossary/topical-authority.md
      case-studies/acme.md
      changelog/2026-10.md
      snippets/affiliate-disclosure.md
    de/
      posts/...
  media/                 originals; derived sizes and formats are generated and cached
  data/site.db           SQLite: the index, versions, forms, subscribers, comments, analytics
  lint.toml              rule severities and thresholds
  facts.toml             the fact registry: values posts repeat, their old values, when last checked
  briefs/<lang>/<slug>.toml  optional plan per post: questions, terms, links the linter checks
  public/                static export output, static mode only
```

### 4.4 Many sites in one process

The program keeps a `sites/` folder. The default install makes one site in it. The super-admin panel lists the sites in this process and can add a domain, which makes a new site folder and starts serving it. Requests are routed by host name. Each site has its own users, theme, content and database. A super admin sees all of them.

### 4.5 Rendering and speed

- Server-side templates with MiniJinja, so users can override a template without recompiling. Pages render in under a millisecond and are cached until a file changes.
- Semantic HTML by default. Every default template uses the element that says what the content is: `header`, `nav`, `main`, `article`, `aside`, `footer`, `time` for dates, `figure` and `figcaption` for images, one `h1` per page with headings in order, labelled landmarks, and `lang` and `dir` on every page. Search engines, screen readers and AI crawlers all read structure first. The default templates pass the W3C validator and an axe accessibility audit with zero errors.
- Every public page loads and works with JavaScript blocked. Pages render complete on the server. Navigation, pagination, the language switcher, search, comment posting and the newsletter form are plain links and HTML forms first. JavaScript only adds polish on top, such as live search results or a copy button, and the page is whole without it. The admin is the one exception, because the editor needs JavaScript.
- Critical CSS inline, the rest in one small file. Fonts self-hosted when the licence allows, with a system fallback otherwise. Images converted to WebP and AVIF with a responsive set of sizes, lazy by default.
- Brotli, ETags, immutable cache headers for assets, security headers, clean URLs.
- Target: 100 in all four Lighthouse categories on the default post page. Largest Contentful Paint under one second on a five-dollar server.
- The full front-end checklist is section 4.10.

### 4.6 The admin

- A small React and TypeScript app, built once and embedded in the binary. It is served under `/admin` by default, and the path can change.
- Pages: Dashboard, Posts, Calendar, Editor, Media, Authors, Collections, Snippets, Forms, Subscribers, Comments, Navigation, Design, Redirects, Lint, Audit, Search Console, API keys, Settings, and Sites for the super admin. The posts table filters by lint status, review state, index status and Search Console numbers, with quick edit and bulk actions for title, description, tags, category and author.
- The editor is a markdown-first rich-text editor built on Tiptap. It stores markdown, and every block has a markdown directive, so the same post can be written in a text file. Blocks: heading, paragraph, image with required alt and caption, gallery, video, audio, file download, bookmark card from a pasted link, product card with rating and button, callout, toggle, button, header, divider, code, quote, table, embed, HTML with visibility rules, FAQ, how-to, review, recipe, business card, form, newsletter signup, gated content, table of contents, footnote, maths.
- Writing aids: a slash menu, markdown shortcuts, emoji autocomplete with `:`, link search with `cmd+k`, `@` to mention a post, tag or author, wikilinks, Unsplash and GIF search, basic image editing with crop, rotate and annotate, snippets that save a group of blocks and update everywhere, a live preview pane in the real theme, autosave, an unsaved-changes guard, version history with diff and restore, and a presence indicator with post locking when someone else is editing.
- Post settings: slug, date, co-authors, custom excerpt, feature image with caption and alt, featured flag, review state, access, custom template, canonical, robots, per-network share cards for X and Facebook, aliases, scheduled unpublish, password, per-post head code, and the Updates list of dated changes.
- A logged-in toolbar on the public site: edit this post, lint status, this post's numbers.
- Accounts: email and password, passkeys, and a magic link. Roles: owner, admin, editor, writer, contributor. SerpDad's `auth` crate already has the passkey code to copy.
- House rules apply: light theme only, one focus point per screen, every decision screen explains its context first, no placeholders. **Mockups get a yes from the founder before any admin screen is built.**

### 4.7 Deploy

- A single file. Install with one line, or Homebrew, or a Docker image built from scratch.
- `khalid serve --domain blog.example.com` gets its own HTTPS certificate through Let's Encrypt and renews it. No reverse proxy is needed for the simple case.
- Ships a systemd unit, a Docker Compose file and one-click templates for Fly.io, Railway, Coolify and a Hetzner VPS.
- Idle memory under 50 MB for one site. Binary under 30 MB.

### 4.8 Mounting under a folder of another site

When the blog must live at `example.com/blog` and `example.com` is built with something else, the main site has to pass `/blog` through. `khalid init` detects this case and prints the rule for the stack the user names:

Nginx, Caddy, Apache, Traefik, Cloudflare Worker, Vercel rewrites, Next.js rewrites, Netlify `_redirects`, Webflow through a Cloudflare reverse proxy, Shopify through a Cloudflare Worker.

The CMS knows its mount path, so every link, feed and sitemap uses the full public address. `khalid doctor` fetches the public address and confirms the rule works.

### 4.9 Theme sync

`khalid theme sync https://example.com` does this:

1. Fetch the home page and its stylesheets.
2. Read CSS custom properties, `@font-face` rules and Google Fonts links.
3. Find the dominant background, text, link and accent colours by frequency and contrast.
4. Find the logo through `og:image`, the header image and the icon links.
5. Write `theme/tokens.toml` and render a preview page.
6. Check every text and background pair for WCAG contrast. A failing pair gets the nearest passing shade, and the user is told.

The Design screen in the admin edits the same tokens by hand: logo, icon, cover image, fonts, colours, social accounts, the announcement bar, and the primary, secondary and footer menus. A change there is a change to `theme/tokens.toml`, so git sees it.

Limits, and what we do about them:

| Problem | Answer |
|---|---|
| The site renders with JavaScript and the HTML is empty | Tell the user, offer `--from-css` to paste their variables, and fall back to a clean default |
| Class names are hashed and colours live in inline styles | The frequency pass still works on the colours; fonts may need a manual line |
| Fonts from Adobe or a commercial foundry | Cannot be re-hosted. Use the system fallback and say so in plain words |
| The main site changes next month | `theme sync` again shows a diff. Fields the user edited by hand are kept |
| Header and footer | Not cloned in v1. The blog carries the logo, a link back to the main site and a configurable set of nav links. Cloning the real header is Phase 8, because it rots |

### 4.10 Front-end defaults

The checklist lives in the repository as `FRONTEND.md`. Every default template follows it, CI checks what a machine can check, and the linter and `khalid doctor` warn when a user's override breaks it.

**Responsive on every device**

- Mobile first. Styles start at a phone width and grow. Breakpoints are added only where the content needs them, not at device sizes.
- Fluid type and spacing with `clamp()`, so text and gaps scale smoothly between a 320 px phone and a 1920 px monitor. Line length stays between 60 and 75 characters.
- No horizontal scroll at any width from 320 px up. Long words and URLs wrap. Tables scroll inside their own box. Code blocks scroll inside theirs. Embeds keep their aspect ratio.
- Images carry `srcset`, `sizes`, `width` and `height`, so the browser picks the right file and reserves the space.
- Touch targets at least 44 by 44 px, with space between them. Nothing depends on hover.
- Correct viewport meta, safe-area insets for notched phones, and layouts that still work at 200% zoom and in landscape.
- Container queries for components that live in more than one place, such as cards and the newsletter block.
- Checked in CI at 320, 375, 414, 768, 1024 and 1440 px wide on every change.

**Accessible**

- WCAG 2.2 AA. Colour contrast is checked against the synced theme tokens, not assumed.
- A skip link, visible focus on everything, full keyboard use, native elements before ARIA.
- Every form field has a label and a plain error message. Every image has alt text or is marked decorative.
- `prefers-reduced-motion` respected. Nothing autoplays.

**Fast and stable**

- Zero layout shift: reserved image space, font fallbacks tuned with `size-adjust`, no late-loading banners.
- Fonts with `font-display: swap`, self-hosted, subset to the languages the site declares, at most two families.
- The hero image gets `fetchpriority="high"`. Everything below the fold is lazy.
- One small CSS file under 20 KB compressed, no CSS framework, no render-blocking JavaScript, no third-party script by default.
- Long cache lifetimes with hashed file names, Brotli, and early hints where the host supports them.

**Robust**

- Valid HTML. Works with scripts off, in reader mode, in RSS readers and when printed, with a print stylesheet.
- Content Security Policy with nonces, no inline event handlers, integrity hashes on anything external, a strict referrer policy.
- Right-to-left layouts mirror correctly. Dates, numbers and quotes follow the page language.

### 4.11 Routing: collections, channels and permalinks

URLs are configured in `routes.toml`, the way Ghost does it in `routes.yaml`. The defaults need no file at all.

- **Collections** own posts and set their URL pattern. `/blog/` with `{slug}` is the default. A site can add `/guides/` with `/guides/{slug}/` and `/news/` with `/news/{year}/{slug}/`, each with its own index page and feed. A post belongs to exactly one collection.
- **Channels** are filtered lists that do not change a post's URL: `/featured/` for featured posts, `/from-the-founder/` for one author's posts. Each gets a feed.
- **Taxonomy prefixes** rename `/tag/` and `/category/` to whatever the site says, such as `/topic/`.
- **Aliases** in a post's front matter list old URLs that redirect here. They feed the redirect manager, so a migration keeps rankings with one line per post.
- Every pattern knows the mount path and the language prefix, so `/de/guides/{slug}/` just works.
- `khalid lint` warns when two collections could claim the same post.

### 4.12 Reader features on the default templates

Added 2026-10-27 after a page-by-page check of the Ahrefs blog, Appendix A.3. Each one works as plain HTML first and adds JavaScript only on top, per 4.10. Each one can be switched off in `site.toml`.

**On the post page**

- **Byline with photo and role.** Every author's avatar, name and job title from the author profile, linked to the author page. Several authors in a row.
- **Reviewed by.** An optional `reviewed_by` field in front matter names one or more authors. The byline shows "Reviewed by" with their names, and the page's JSON-LD carries `reviewedBy`. It is an E-E-A-T signal, and the linter can require it per category, for example for health or money posts.
- **Reading time.** "8 min read" next to the date, from the word count at about 230 words a minute. Languages written without spaces, such as Japanese and Chinese, count characters instead. The strings follow the page language.
- **Table of contents.** Built from the post's H2 and H3 headings, on by default when a post has three or more H2s. On wide screens it sits in a sticky side column and marks the section being read; on phones it is a collapsible `details` block above the body. The `::toc` block still places one inside the body.
- **A link on every heading.** Each H2 and H3 gets an anchor link, visible on keyboard focus and always reachable on touch. With JavaScript, a click copies the address and says so.
- **Click to enlarge.** A body image that is shown smaller than its original links to its largest variant. With JavaScript, it opens in a native `dialog` with its caption, closes with Escape, and keeps focus correct.
- **Author box at the end.** Photo, name, role, bio and profile links for each author, then the reviewer's name. Replaceable through the author-box snippet, 5.1.
- **Related posts.** Three to five posts under the body, chosen at build time from shared category, shared tags and wikilinks in both directions, newest first on ties. Deterministic, so the same build gives the same list. A post can pin its own list in front matter.

**On lists and the home page**

- **Richer post cards.** Author name and avatar, date, primary category and title, in that order, on every list.
- **Home page sections.** `site.toml` can list categories, collections or channels for the home page. Each gets a section with its newest posts and a "View all" link. The default stays one newest-first list.
- **Topic links.** A row of links to every category, grouped the way the menu groups them, under the home page sections.

**Site-wide**

- **Authors page.** `/authors/` lists every author with photo, role and post count. Each author page ends with the other authors.
- **Archive.** `/archive/` lists every published post across collections, newest first, with pagination. It is `noindex` when it repeats the home page exactly.
- **Search shortcut.** A search link in the header goes to the search page. With JavaScript, `/` and `Ctrl+K` or `Cmd+K` open the search box from any page.

Not added: the "linking websites" count Ahrefs prints on each post. It needs backlink data, which section 5.11 leaves to SerpDad, Ahrefs and Semrush. A SerpDad-connected site may show it in Phase 7.

---

## 5. Features at launch

### 5.1 Writing

Drafts, scheduled publish, preview links that expire, revisions through git, authors with bios and `sameAs` links for E-E-A-T, categories, tags, series, related posts, table of contents, reading time, code blocks with syntax colours, image handling as above, embeds, callouts, FAQ blocks, footnotes, last-updated dates.

Also: a focus topic per post, one primary keyword and its related terms, with checks in the linter. A Google snippet preview. An SEO score out of 100 that sums up the lint checks, see D-10. Pillar posts and their clusters. A primary category for breadcrumbs. Blocks for FAQ, how-to, review, recipe, video and a business card with address, hours and map, each of which writes its own schema. Images get alt and title suggestions from the caption or the filename, a bulk find-and-replace for alt text, clean filenames and stripped EXIF data. A post can be a folder with its images beside the markdown. Wikilinks `[[slug]]` resolve at build and are linted when broken. Every post shows linked mentions, the posts that link here, and internal links get a hover preview that degrades to a plain link with scripts off. Maths renders on the server as MathML. Each post can carry an Updates list of dated changes, aliases for old URLs, a scheduled unpublish date and a password. Snippets hold reusable groups of blocks, such as an affiliate disclosure or an author box, and update everywhere at once.

### 5.2 SEO defaults

| Feature | What it does | What it replaces in WordPress |
|---|---|---|
| Titles and descriptions | Templates per page type with variables such as the title, the site name and a separator. Length and duplicate checks across the site. A Google snippet preview in the editor. Bulk edit in the posts table | Yoast, Rank Math |
| Canonical, Open Graph, Twitter cards | Always present, always correct for the mount path and language. A default share image, a per-post override, and a share image generated from the title and the theme colours when no image is set | Yoast, Rank Math Social |
| JSON-LD | Every type in section 5.9, written from the content and validated before publish | Rank Math Schema, Schema Pro |
| Sitemaps | Posts, pages, images, video and news, per language with alternates, split at 50,000, listed in robots.txt, plus an HTML sitemap page. Any post can be excluded | Yoast, XML Sitemaps |
| Feeds | RSS, Atom and JSON Feed, per language and per category, full or excerpt, with optional text before and after each item and a source line that names the original URL | Core WP, Rank Math RSS |
| robots.txt and AI crawler controls | Sane defaults, plus `llms.txt` and per-bot allow or block | Manual |
| Redirects | 301, 302, 307, 410 and 451, regex rules, CSV import and export, an automatic 301 whenever a slug changes, chains collapsed automatically, and a debug view that shows which rule matched | Redirection plugin, Rank Math |
| 404 log | Every 404 recorded, with a one-click redirect to the nearest post | Redirection plugin |
| Breadcrumbs and pagination | Correct markup and `rel` links | Yoast |
| Thin archives | Tag and author pages with few posts are `noindex` by default | Yoast settings |
| Index pings | IndexNow on publish; sitemap ping to Google and Bing | Plugins |
| hreflang | Automatic across translations, with `x-default` | WPML, Polylang |
| Speed | 100 Lighthouse by construction | WP Rocket, image plugins |
| Semantic HTML | The right element for every part of the page, valid and accessible | Depends on the theme |
| Works without JavaScript | Every page and every form works as plain HTML | Depends on the theme and on every plugin |
| Mobile-first | Responsive on every screen, passes Google's mobile usability checks | Depends on the theme |
| Security headers, HTTPS, HSTS, www and trailing-slash policy | Set once in `site.toml` | Several plugins |
| Robots and canonical per post | `noindex`, `nofollow`, `noarchive`, `nosnippet`, `max-snippet`, `max-image-preview`, `max-video-preview`, and a canonical override for syndicated posts. Index settings per archive type: tags, categories, authors, dates, search results | Yoast, Rank Math |
| Link policy | Site-wide defaults for external links: `nofollow`, `sponsored` or `ugc`, open in a new tab, and a list of affiliate domains that always get `sponsored` | Rank Math, Pretty Links |
| Site verification | Meta tags for Google, Bing, Yandex, Baidu and Pinterest, and any verification file served from the root | Rank Math |
| Head and body code | Custom code per site and per post, for a tag manager or analytics, allowed through the Content Security Policy, with a consent banner that holds scripts until the visitor agrees | Header Footer Code Manager, consent plugins |

### 5.3 The linter

See section 6.

### 5.4 Multilingual

- A language folder per language under `content/`. A post names its translation group in its header with `translation_of`.
- URLs: `/de/slug` by default; a subdomain per language is an option.
- hreflang on every page, with `x-default`, and sitemap alternates. `khalid lint` warns when a translation group is missing a language the site declares.
- Right-to-left support in the default templates. Dates and numbers in the page language.
- A language switcher that links to the translated post, not the home page.

### 5.5 Server-side features

Full-text search with SQLite FTS5. Comments with moderation and an optional Giscus mode. Newsletter capture with double opt-in and sends through SMTP, Resend or Postmark. Privacy-friendly analytics with no cookies, counted on the server, so it works with scripts blocked too, with referrers from ChatGPT, Perplexity, Gemini, Copilot and Claude grouped as AI search traffic. A crawl log that shows which bots fetched which page and when, including Googlebot, Bingbot and the AI crawlers, which is what Semrush sells as its Log File Analyzer. CTA blocks with UTM tags. Webhooks for Zapier and Make. Scheduled publish. Preview links. IndexNow. The redirect manager and the 404 log. Alerts by email or webhook for broken links, 404 spikes, certificate problems, failed scheduled posts and stale posts. CSV and JSON export of every table, so Looker Studio and spreadsheets can read it. Form submissions stored with spam protection and delivered by email and webhook. Subscriber segments and labels, several newsletters, open and click analytics, and a welcome email with a short sequence after signup. Gated content that unlocks with an email address. An embeddable signup form for the main website. An oEmbed endpoint, so other sites embed posts as cards. The Bluesky verification file at `/.well-known/atproto-did`. An audit log, a trash with restore, and bulk actions. Git mode: each draft can be a branch and a pull request, publishing is the merge, and the blog rebuilds when the repository is pushed.

### 5.6 Growth features

CTA and newsletter blocks, a form builder block with a submissions table and delivery by email and webhook, gated content that unlocks with an email address, subscriber segments, several newsletters with open and click analytics, a welcome sequence, an embeddable signup form for the main website, an announcement bar, a UTM builder for shared links, social card preview in the editor, webhooks, RSS to email. Auto-posting to social networks is Phase 8.

### 5.7 Migration

One command per source. Each one imports posts, pages, authors, categories and tags; downloads and rehosts images; keeps every slug; writes a redirect map for any URL that must change; runs as a dry run first; prints a report of what came in and what did not. The WordPress import also carries the SEO plugin data: titles, descriptions, focus keywords, canonicals, robots settings, schema and redirects from Yoast, Rank Math, All in One SEO, SEOPress and the Redirection plugin.

| Source | How | Phase |
|---|---|---|
| WordPress | WXR export file, or the REST API with an application password | 6 |
| Ghost | JSON export | 6 |
| Medium | Export archive | 6 |
| Substack | Export archive | 6 |
| A folder of markdown | Hugo, Jekyll, Astro and Zola conventions | 6 |
| Webflow | CSV export | 8 |
| HubSpot | Export file | 8 |

### 5.8 For agents

Everything the command line does, the MCP server and the Admin API do too, and every block has a markdown directive, so an agent can write a complete post with cards in one text file. `--json` everywhere. The repository ships an `AGENTS.md` and a Claude Code skill, so "add a blog to my site" works out of the box. Lint output is shaped so an agent can fix and retry.

### 5.9 Schema markup

Schema comes with the CMS. Nothing to install, nothing to paste. The CMS writes JSON-LD for every page from the content it already has, validates it before publish, and lets advanced users add any type schema.org knows.

**Written for every site, from `site.toml`**

- Organization or Person as the publisher, with logo, social profiles as `sameAs`, contact and founding details. The admin asks once: "Is this site a company or a person?"
- LocalBusiness, optional, with the business type from the full schema.org list, address, opening hours, phone, price range and map link. One location or several.
- WebSite with a SearchAction in server mode, so Google can show a sitelinks search box.
- BreadcrumbList on every page.

**Written for every post, from the content**

| From | Schema |
|---|---|
| Any post | Article, BlogPosting, NewsArticle or TechArticle by content type, with headline, images, author, publisher, dates, word count, language and `mainEntityOfPage` |
| The author profile | Person with job title, bio, `sameAs` and `knowsAbout` |
| An FAQ block | FAQPage |
| A how-to block | HowTo with steps, tools, supplies, time and cost |
| A video embed | VideoObject, filled from the video's own data: title, thumbnail, duration, upload date |
| A review block | Review and AggregateRating, with the item reviewed |
| A recipe block | Recipe |
| A listicle | ItemList of the items, and Carousel on list pages |
| An event, course, product, software, book or podcast block | Event, Course, Product, SoftwareApplication, Book, PodcastEpisode |
| A fact-check block | ClaimReview |
| A dataset block | Dataset |
| The summary or intro | Speakable |
| Linked entities | `about` and `mentions`, from the entities the post links to. SerpDad's briefs already know these entities, so a post published from SerpDad arrives with them filled |

**For advanced users**

- A schema template per category or content type, so every post in "Recipes" is a Recipe without anyone thinking about it.
- A custom schema builder that offers every schema.org type and property, with a form generated from the vocabulary. Rank Math calls this "840+ types". The vocabulary is open and ships inside the binary.
- A custom JSON-LD field per post and per site for anything the builder does not cover.
- "Copy schema from a URL": fetch any page's JSON-LD as a starting template.
- A schema preview in the editor that shows exactly what the page will emit, and a one-click link to Google's Rich Results Test.

**Checked**

- Every graph is validated against the schema.org vocabulary and Google's required and recommended properties before publish. Missing required properties block, see D-9. Missing recommended ones warn.
- The site audit, section 5.10, reports schema per type: how many pages carry it, how many are valid, and which rich results they are eligible for.

### 5.10 Built-in site audit and health score

Ahrefs and Semrush both crawl a site and grade it. Ahrefs checks about 170 issues, Semrush about 140, and both give a health score out of 100. The default templates pass all of those checks by construction. The audit exists to prove it, to catch what a user's override or content breaks, and to give the dashboard the same score people know from those tools.

- `khalid audit` crawls the site the way Ahrefs and Semrush do: from the home page, following links, reading robots.txt, the sitemaps and hreflang. It runs nightly in server mode and in CI.
- Checks are grouped the way those tools group them, so an SEO recognises them: Crawlability and architecture, Indexability, Titles and descriptions, Headings, Content and duplicates, Internal links, Outgoing links, Redirects, Images, Resources, HTTPS and security, Localization and hreflang, Social tags, Sitemaps and robots, Structured data, Performance and Core Web Vitals, AI search.
- Every check Ahrefs or Semrush names has a counterpart. Examples the default templates cannot break but overrides and content can: redirect chains and loops, a canonical pointing to a redirect or a 404, noindex or redirected pages in the sitemap, hreflang without a return tag, pages more than three clicks from home, pages with one inbound link, nofollow internal links, more than a configurable number of links on a page, near-duplicate posts, low text-to-HTML ratio, HTML over a size limit, missing HSTS, links from HTTPS pages to HTTP, outdated posts, low semantic HTML use, llms.txt format.
- Core Web Vitals come from two places: lab numbers from Lighthouse in CI, and real-user numbers from Google's CrUX API per page, opt-in and free.
- The result is a health score out of 100 with a trend chart, the issues by severity with the pages affected, a fix instruction for each, and CSV export. The dashboard shows the score.
- The audit also writes the Internal linking report: click depth per post, inbound and outbound counts, an internal link rank, link distribution, orphans, broken and redirecting links, and suggestions. Semrush sells this as a thematic report. Here it is a page.

### 5.11 Search Console connection

Rank Math's most used module is its Analytics module: Search Console data inside the editor. The CMS gets the same, opt-in, using the user's own Google account. SerpDad is never involved unless the user connects it. See D-8.

- Per post: clicks, impressions, average position and click-through rate for the last 28 days and over time, with the queries behind them. Shown in the posts table and on the post's page in the admin.
- Index status per post from the URL Inspection API, within Google's daily quota.
- Winners and losers: the posts and queries that gained or lost the most, every week.
- Decay: a post whose clicks fell for three months in a row is flagged on the dashboard, with "Refresh this post" as the action.
- 404 and noindex URLs that still receive impressions or clicks, from the 404 log joined with Search Console. Ahrefs and Semrush both list these as errors.
- A weekly email report with the score, the winners and losers, new issues and what to do next, in plain words.
- Everywhere a number asks a question the CMS cannot answer, such as "why did this drop" or "what should I write next", a link reads "Go deeper in SerpDad". That link is the whole funnel, and it is the only place SerpDad appears without the user asking.

Rank tracking, keyword research, backlinks, competitors and AI visibility stay out of the CMS. Those are SerpDad's job, and the job of Ahrefs and Semrush.

### 5.12 Public API, webhooks and headless mode

Everything the admin does goes through the same API, and the API is public and documented.

- **Content API**, read-only, for the public content. Posts, pages, authors, tags, categories, collections, glossary, case studies, changelog, site settings and navigation. Ghost's query shape, because developers already know it: `filter` with the same syntax, `include`, `fields`, `formats` for `html`, `markdown` and `plaintext`, `limit`, `page`, `order`. Keyed with a content key that is safe in a browser. Fully cacheable, with ETags.
- **Admin API**, read and write, for everything else. Create a post from markdown, or from HTML with `source=html`. Upload images and files. Manage tags, authors, redirects, snippets, forms, subscribers, webhooks and settings. Keys per integration, staff tokens per user, scopes per key. Edits carry `updated_at`, and a stale edit is refused rather than overwriting someone's work.
- **Webhooks** for named events: `post.published`, `post.published.edited`, `post.unpublished`, `post.scheduled`, `post.deleted`, the same for pages and collection entries, `tag.added`, `form.submitted`, `subscriber.added`, `comment.added`, `lint.blocked`, `audit.finished`, `site.changed`. Every delivery is signed with HMAC and retried with backoff. A delivery log in the admin.
- **OpenAPI** specification served by the binary, a TypeScript SDK generated from it, and `Accept-Version` headers so old clients keep working.
- **Headless mode.** The theme can be switched off. The CMS still runs the editor, the linter, the schema, the sitemaps, the feeds and the API, and another app renders the pages. Two starter kits, Astro and Next.js, render a complete blog from the Content API, with the schema and meta tags carried through.
- The SerpDad publish target, the MCP server and the command line are all clients of this API. There is no second door.

### 5.13 Collections: glossary, case studies, changelog

Three fixed collections beside posts and pages, each with its own folder, list page, templates, schema and lint rules. Decided in D-12. Not a general content-type builder.

| Collection | Why | Schema | Lint |
|---|---|---|---|
| Glossary | Definitions rank, earn links and anchor topical authority. Every SEO wants one | DefinedTerm in a DefinedTermSet, with FAQPage when a term has questions | A post that uses a glossary term without linking it gets a warning. A term with no inbound links is an orphan |
| Case studies | The business blog's proof pages | Article with `about` the client's Organization, plus Review or AggregateRating when there is a quote with a rating | Needs a client, an outcome and a date |
| Changelog | Product updates in one place, with a feed people subscribe to | TechArticle entries, grouped by month, with `dateModified` on the product page | Each entry needs a date and a version or a month |

Each collection has typed front matter, validated on save, and a list page with pagination, search and its own feed. The glossary also gets an A to Z index.

### 5.14 Collaboration and workflow

- **Review states.** Draft, in review, ready, published. Roles decide who can move a post between states: a writer submits, an editor approves. The states are also API fields and MCP operations, so SerpDad can stage into "in review".
- **Comments on drafts.** Threaded, attached to a block, with email notifications and a resolve button.
- **Calendar.** Drafts, scheduled posts and published posts by day. Drag to reschedule.
- **Git mode.** Optional. Each draft is a branch named after the post and a pull request. Publishing is the merge. The blog pulls on push through a webhook or `khalid pull`. Developers write in their own editor and ship posts like code.
- **Presence and locking.** Who else has this post open, and a lock that releases when they leave.
- **Audit log** of who changed what and when, across content, settings and users. **Trash** with restore. **Bulk actions** on the posts table.

### 5.15 Media library

Folders, search, duplicate detection on upload, and a focal point per image that drives every crop and every size. Alt text and caption live on the image and follow it into every post. Storage on disk by default, or on any S3-compatible bucket such as Cloudflare R2. Unsplash and GIF search from the editor. Basic editing: crop, rotate, annotate. Every derived size and format is generated once and cached.

---

## 6. The linter

The rule of the linter: **every message says what is wrong, why it matters in one plain sentence, and how to fix it.** It links to a SerpDad guide for the long version. This is how the free tool teaches SEO and brings people to SerpDad.

Severities come from `lint.toml`. The defaults:

### Blocks: the post cannot be published

| Rule | Message |
|---|---|
| No publish date | "This post has no date. Google and readers use dates to judge freshness. Add `date:` to the header." |
| No author | "This post has no author. People and search engines trust content with a named writer. Add `author:` and make sure that author has a profile." |
| No internal link | "This post links to none of your other posts. Internal links are how readers and Google find the rest of your site. Add one link to a related post." Shows three suggestions. |
| No title, or more than one H1 | "A page needs exactly one headline." Proposed, see D-3 |
| Invalid schema | "The structured data on this page is not valid, so Google will ignore it." Names the property. Proposed, see D-9 |

### Warnings: shown, publish still allowed

| Rule | What it catches |
|---|---|
| Anchor text reused for a different page | The same anchor text pointed at another post before. Google reads anchor text as a label: one label, one page |
| Same anchor to the same page too often | The same exact anchor text to the same URL more than N times across the site |
| Vague anchors | "click here", "read more", "here", "this post" |
| Anchor misses its target | The anchor text shares no word with the title, search title or focus keyword of the post it opens |
| Anchor is the post's own topic | An internal link to another page whose anchor text holds this post's own focus keyword |
| Same target linked twice in one post | Only the first link counts |
| Link to a redirecting or dead URL | Internal or external |
| Orphan post | A published post with no inbound internal link. Checked after publish and shown on the dashboard |
| Heading order | H2 to H4 with no H3 |
| Title or description length | Too long for the result snippet, or missing |
| Duplicate title or description | Another post on the site has the same one |
| Missing alt text | Any image without alt |
| Thin post | Under a configurable word count |
| Keyword stuffing | One phrase over a density threshold |
| Hard to read | Reading grade above a threshold |
| Slug hygiene | Too long, stopwords, dates in the slug |
| Missing translation | A language the site declares has no version of this post |
| Cross-site linking | In a multi-site process, a post that links to a sibling site. Google reads sibling sites linking to each other as a network |
| No outbound link | Posts that cite nothing |
| Non-semantic markup | A template override or a raw HTML block that uses `div` or `span` where `article`, `nav`, `time` or `figure` belong, or a heading made from styled text |
| Needs JavaScript | A template override or embed that breaks with scripts off: a form without an `action`, a link that goes only to `#`, or a `noscript` notice asking to turn JavaScript on |
| Oversized image | An image far wider than any slot it can render in, or over a size threshold before conversion |
| Fixed width | A template override or raw HTML with a fixed pixel width wider than a phone screen |
| Focus topic missing | The primary keyword is absent from the title, the H1, the URL, the first paragraph, the description or every heading. One message per place, and a stuffing warning when it appears too often |
| Cluster without its pillar | A post in a cluster that does not link to its pillar post |
| Nofollow internal link | An internal link with `nofollow`. Internal links should always pass |
| Too many links | More than a configurable number of links on one page |
| One inbound link | A published post with only one inbound internal link |
| Too deep | A post more than three clicks from the home page |
| Link without anchor text | An image link without alt text or a label |
| Stale post | Not updated in a configurable number of months. Shown on the dashboard with "Refresh this post" |
| Near-duplicate | Two posts that share most of their text, measured with shingles |
| Insecure internal link | An internal link that starts with `http://`. Fixed automatically |
| Broken wikilink | A `[[slug]]` that matches no post, page or glossary term |
| Glossary term not linked | The post uses a term the glossary defines without linking it. Shown with a one-click fix |
| Form without a destination | A form block that delivers to no email address and no webhook |
| Overlapping collections | Two collections in `routes.toml` could claim the same post |
| Old value of a fact | The post gives a value `facts.toml` lists as one a fact held before. Suggests the current value |
| Fact not checked | The post gives a fact from `facts.toml` whose `checked` date is older than a configurable number of months, default 6 |
| Brief question unanswered | The post's brief lists a question no heading asks, in any word order |
| Brief term missing | The post's brief lists a term the body and headings never use |
| Brief link missing | The post's brief lists a page the post does not link to |

`khalid lint --fix` repairs the safe ones: alt text from the caption, slug cleanup, heading order. Everything else is for the writer.

The rule engine is a crate of its own, with one test per rule and one fixture per message, so SerpDad's backend can run the same rules on its own drafts.

---

## 7. Server mode and static mode

Static mode writes plain HTML to `public/` for people who want to serve the blog from their existing host with no process running. The admin still runs on their machine, and `khalid export` runs after each change.

| Feature | Server | Static |
|---|---|---|
| Posts, pages, authors, feeds, sitemaps, schema, hreflang | Yes | Yes |
| Theme sync, lint, import, CLI, MCP | Yes | Yes |
| Web editor | Yes | Yes, locally |
| Scheduled publish | Yes | Locked: "Static sites change only when you export. Run `khalid export` on a schedule, or switch to server mode." |
| Search | Yes | Locked, with a client-side option on small sites |
| Comments | Yes | Locked, Giscus option |
| Newsletter capture and sends | Yes | Locked, with a form that posts to a provider |
| Redirects and the 404 log | Yes | Redirects exported as a `_redirects` or Nginx file; no log |
| Analytics | Yes | Locked |
| Preview links | Yes | Locked |
| Publish API for SerpDad | Yes | Locked: "SerpDad needs a running server to deliver drafts. Switch to server mode, or download the article and add it to `content/`." |
| Content API | Yes | Read-only, exported as static JSON files beside the pages |
| Admin API, webhooks, API keys | Yes | Locked: "Writing through the API needs a running server. Switch to server mode, or edit the files in `content/`." |
| Forms and gated content | Yes | Locked, with a form that posts to a provider instead |
| Subscriber segments and email sequences | Yes | Locked |
| Live-site toolbar, presence, draft comments | Yes | Locked. The admin still runs locally |
| Git mode | Yes | Yes |
| Glossary, case studies, changelog | Yes | Yes |

Every lock is a grey card with the feature's name, one sentence on why it needs a server, and the one command that turns on server mode.

---

## 8. The SerpDad side

- **Free tools page.** A page on serpdad.com with the pitch, a one-line install, a demo blog and the GitHub link.
- **Dashboard.** A tile under integrations: "Your blog, by SerpDad". Connect with the blog's address and a token made in the CMS admin under Settings, SerpDad.
- **Publish with khalid.** In the content journey, next to the existing draft targets. Sends the finished article the same way as WordPress today: title, body, slug, excerpt, as a draft. The CMS runs its linter on arrival and returns the result, so the SerpDad screen can say "Staged, 0 blocks, 2 warnings" and link to the post in the admin.
- **Backend work in this repo.** A `khalid` module in the Rust `integrations` crate and a `DraftTarget` for it, a `CmsConnection` provider, and the connect screen. Follows the WordPress one line for line.
- **Docs.** Guides on serpdad.com for every lint rule, since the rules link there.

---

## 9. Licence, name and trademark

- **AGPL-3.0** means anyone may use, change and host the CMS. Anyone who changes it and lets others use it over the network must publish their changes. That stops a closed fork.
- **What AGPL does not stop:** a host taking the unchanged program and selling it as a service. Two things close that gap. The **name is a trademark**: the licence covers the code, not the name, so a reseller must rename it and lose the "by SerpDad" trust. Plausible and Ghost work this way. The **contributor licence agreement** keeps SerpDad free to use every contribution in its own paid products.
- **Cost of AGPL:** some company legal teams ban AGPL software outright, even unmodified. The README gets a plain FAQ: "You can use it for your blog. You owe nothing unless you change the code and run that changed version for others."
- **Name.** Phase 0 picks it. Checks before choosing: GitHub organisation, crates.io, npm, the `.dev` and `.com` domains, a trademark search in the US and EU, and no existing CMS or editor with the same name.

---

## 10. Phases

Each phase ends with something the founder can see. Builds and tests run on `hetzner-box` or CI, not on the laptop.

### Phase 0. Name, repo, rules (1 week)
Pick the name. Create the repository, licence, CLA bot, trademark note and code of conduct. Cargo workspace with the same toolchain, lints, `deny.toml` and CI as the SerpDad Rust workspace, plus the rule catalogue from the code-quality plan. An empty binary that prints help. **Shows:** a repo people could star.

### Phase 1. The engine and the API (3 weeks)
Content model, markdown pipeline with a directive for every block, wikilinks, co-located assets, maths as MathML, semantic, responsive templates that work with scripts off, server, SQLite index, feeds, sitemaps, robots, schema, clean URLs, language folders, `routes.toml` with collections, channels, permalinks and aliases, the glossary, case studies and changelog collections, the reader features from 4.12, the Content API and Admin API with keys, webhooks with signing, the OpenAPI spec and the TypeScript SDK. `init`, `new`, `serve`, `build`, `keys`. **Shows:** the SerpDad blog served locally from markdown files, 100 on Lighthouse, and the same posts fetched through the Content API.

### Phase 2. The linter, schema and SEO defaults (3 weeks)
Every rule in section 6 with tests and messages. `lint.toml`. The publish gate. The schema engine from section 5.9. Redirects, 404 log, IndexNow, hreflang, `doctor`, and the site audit from section 5.10. **Shows:** `khalid lint` blocks a post and explains itself, and `khalid audit` scores the demo site 100.

### Phase 3. Theme sync and mounting (1 week)
Token extraction, fonts, logo, preview, re-sync diff. The proxy rule generator for the stacks in 4.8. `khalid theme check` for template overrides. **Shows:** run against ten real sites, eight look native without edits.

### Phase 4. Admin and editor (5 weeks)
Mockups first, the founder's yes, then build. Accounts and passkeys. The editor from section 4.6 with every card, snippets, link search, Unsplash, image editing, autosave, version history, live preview, presence and locking. Review states, draft comments and the calendar. The media library with folders, focal points and S3 storage. The form builder. Navigation, Design and the announcement bar. API keys, audit log, trash, bulk actions. The collections and snippets screens. The logged-in toolbar on the public site. Media, authors, redirects, lint page, settings, super-admin sites panel. **Shows:** a marketer publishes a post with a bookmark card and a form from the browser, hits a lint block, fixes it, and sees the post in the calendar.

### Phase 5. Server features and static mode (2 weeks)
Search, comments, newsletter, analytics, webhooks, scheduled publish, preview links, form delivery, each as a plain HTML form first and JavaScript on top. Subscriber segments, several newsletters, open and click analytics, the welcome sequence, gated content, the embeddable signup form, the oEmbed endpoint, the Bluesky verification file, scheduled unpublish, password pages. Git mode with `pull` and the push webhook. The Search Console connection, alerts and the weekly report. `export`, the lock cards, the static JSON Content API, built-in HTTPS. **Shows:** the same site in both modes.

### Phase 6. Migration (1 to 2 weeks)
WordPress, Ghost, Medium, Substack, markdown folder. **Shows:** a 500-post WordPress blog imported with every slug kept and a complete redirect map.

### Phase 7. SerpDad integration and launch (1 to 2 weeks)
The SerpDad draft target as a client of the Admin API, with its token. The `DraftTarget` and connect screen in this repo. The Astro and Next.js starter kits. Free tools page, docs for every rule and every API endpoint, MCP server, `AGENTS.md` and the Claude Code skill. serpdad.com/blog moves onto it. Release 1.0 with binaries for Linux, macOS and Windows. **Shows:** "Publish with khalid by SerpDad" works end to end on the live site, and a starter kit renders the same blog headless.

### Phase 8. After launch
Header and footer cloning, Webflow and HubSpot imports, auto-posting to Bluesky, Mastodon, LinkedIn and X and then the full social web through ActivityPub, cross-posting to Dev.to and Hashnode with a canonical back, member accounts and paid tiers through Stripe if users ask, Mermaid diagrams, Releases that publish many changes at once, visual editing on the live page, theme packaging and a theme directory, white-label admin and client management for agencies, the admin in other languages, a GraphQL API, client-side search for static sites, a podcast feed, a hosted version if demand shows up, and a public roadmap for everything else.

Timeline: phases 1 to 3 are one line of work. Phase 4 can start after Phase 1. Phases 5 and 6 can run beside Phase 4. With two or three agents, 1.0 lands about thirteen weeks after Phase 0.

---

## 11. How we know it worked

| Measure | Target |
|---|---|
| Install to first published post, a developer following only the README | Under 10 minutes, timed with a fresh agent |
| Lighthouse on the default post page | 100 in all four categories |
| Largest Contentful Paint on a five-dollar server | Under 1 second |
| Idle memory, one site | Under 50 MB |
| Binary size | Under 30 MB |
| Theme sync on ten real sites | Eight look native with no manual edit |
| WordPress import, 500 posts | Zero lost posts, every slug kept, complete redirect map |
| Lint rules | Every rule has a test, a plain message and a guide link |
| Semantic HTML | Every default template passes the W3C validator and an axe accessibility audit with zero errors |
| JavaScript blocked | Every page type readable and every form usable with scripts off, checked in CI on every change |
| Responsive | No horizontal scroll and all text readable from 320 px to 1920 px wide and at 200% zoom, checked in CI at six widths |
| Accessibility | WCAG 2.2 AA on every default template |
| Layout shift | Cumulative Layout Shift of 0 on post pages |
| Weight | Under 20 KB of compressed CSS for the whole site, zero render-blocking JavaScript |
| SerpDad's own blog | Live on it at 1.0 |
| Publish from SerpDad | A finished article arrives as a linted draft in under 5 seconds |
| Ahrefs and Semrush Site Audit on the demo blog | Health score 100, zero errors, zero warnings |
| Rich results | Every schema type the CMS writes passes Google's Rich Results Test |
| Headless | The Astro starter kit renders the demo blog from the Content API in under 10 minutes, with schema and meta tags intact |
| Round trip | Every editor block converts to its markdown directive and back with no loss, tested per block |
| Forms | A submission reaches email and webhook in under 5 seconds |
| Webhooks | Every event in section 5.12 fires once, signed, and retries on failure |

---

## 12. Decisions for the founder

Each one has a recommendation first. "Yes to all" is a valid answer.

- **D-1. Name.** Decided: Khalid, chosen by the founder on 2026-10-26. The original shortlist was Byline, Masthead, Dateline, Gazette and Typeset; the checks and the path to the decision are in `docs/decisions/`.
- **D-2. Many sites.** Reading of answer 4: one program can serve many sites, the default install is one, and the super-admin panel adds domains to the same process. Recommend yes. The other reading, one process per site with a panel that talks to other servers, is more work and not needed for v1.
- **D-3. Two extra blocks.** Block when a post has no title, or more than one H1. Recommend yes. Both make the page wrong, not just weak.
- **D-4. Comments.** Build our own with moderation, or ship only the Giscus option in v1. Recommend our own, because every WordPress migration brings comments along.
- **D-5. Admin look.** Follow the SerpDad shell rules, light theme and Inter, with the CMS's own accent colour, or match the SerpDad app exactly. Recommend the first, since the admin sits on other people's domains.
- **D-6. Contributor agreement.** Recommend yes. It is one click on the first pull request.
- **D-7. Built-in HTTPS.** Recommend yes for v1. It removes the reverse proxy from the simple install.
- **D-8. Search Console inside the CMS.** An opt-in connection with the user's own Google account: per-post numbers, index status, decay, a weekly email. Recommend yes. It is Rank Math's most used module, and every "Go deeper in SerpDad" link hangs off a real number.
- **D-9. Invalid schema blocks publish.** Recommend yes. The CMS writes the schema, so the block only ever fires on custom JSON-LD a user typed, and Google ignores invalid schema anyway.
- **D-10. An SEO score out of 100 per post.** A summary of the lint checks, shown next to the title. Recommend yes, with one rule: the score never replaces the explanations, it points at them.
- **D-11. Adopt the "add to v1" list in Appendix B.** Decided yes, 2026-10-26. Folded into sections 4 to 11. It added three weeks: one to Phase 1 for the API and routing, two to Phase 4 for the editor, workflow, media and forms.
- **D-12. Extra collections beyond posts and pages.** Decided yes, 2026-10-26: a glossary, case studies and a changelog as a fixed set of three typed collections, section 5.13. Not a general content-type builder, which would turn the blog engine into a whole CMS.

---

## 13. Out of scope for 1.0

A page builder. Building the whole website. E-commerce. A themes marketplace. A plugin API beyond webhooks. AI writing inside the CMS, which is SerpDad's job. Keyword research, rank tracking, backlink data, competitor analysis and AI visibility tracking, which are the job of SerpDad, Ahrefs and Semrush. Paid memberships, the social web, GraphQL and visual on-page editing, all Phase 8. A general content-type builder. A hosted version. A mobile app.

---

## 14. Risks

| Risk | What we do |
|---|---|
| "Every feature" never ships | Section 5 is the list. Everything else goes on a public roadmap and waits for 1.0 |
| Theme sync fails on JavaScript-rendered sites | A clear message, `--from-css`, a clean default. A hosted render service later |
| The editor loses formatting on the way to markdown | A defined markdown subset plus named blocks; round-trip tests on every block |
| AGPL scares companies away | A plain FAQ in the README; nobody who uses it unmodified owes anything |
| The name is taken | Check all six places before announcing |
| Spam in comments and forms | Moderation queue, rate limits, a honeypot, optional Akismet |
| A public server gets attacked | `cargo deny` audits, CSP, rate limits, passkeys, signed upload URLs, no file execution |
| Open-source upkeep eats time | Issue templates, agent-assisted triage, a monthly release train |
| Rust slows the admin | The admin is TypeScript; Rust only serves it |
| The editor grows past Phase 4 | Mockups first, every card has a directive and a round-trip test, and anything not in section 4.6 waits |
| API changes break clients | `Accept-Version` headers, the OpenAPI spec as a contract test in CI, and deprecations announced one minor version ahead |
| "Best practices" drift as the web changes | The checklist is `FRONTEND.md` in the repo, pinned to named standards, reviewed once per release |

---

## 15. First steps after approval

1. The founder picks the name from D-1, or gives another.
2. An agent creates the repository with licence, CLA, CI and the empty binary. One pull request.
3. Phase 1 starts. First visible result: the SerpDad blog served from markdown files, within a week.

---

## Appendix A. Coverage check against Rank Math, Ahrefs and Semrush

Checked 2026-10-26 against Rank Math's pricing comparison table, 199 rows; the Ahrefs Site Audit product page and its issues help collection; and the Semrush Site Audit issues list, 41 errors, 34 warnings and 25 notices. Sources are at the end of this appendix.

### A.1 Rank Math, module by module

| Rank Math | In the CMS | Where |
|---|---|---|
| Setup wizard, optimal settings pre-selected, compatibility check | Yes | `khalid init` and `doctor` |
| Titles, descriptions, separator, global meta templates, bulk edit, Google preview | Yes, added | 5.2 |
| Focus keyword, unlimited keywords, content analysis, SEO score | Yes, added as the focus topic plus a score | 5.1, 6, D-10 |
| Google keyword suggestion, search intent analysis, Content AI, competitor analysis | No. SerpDad | 8 |
| Pillar posts, internal linking suggestions, link counter, orphan detection | Yes | 5.1, 5.10, 6 |
| Robots meta per post, canonical, noindex archives, primary category | Yes, added | 5.2 |
| Schema: 18 free types, 6 Pro types, 840+ through the builder, custom JSON-LD, templates, import from a URL, validation, Speakable, video auto-detect, FAQ and HowTo from blocks | Yes, all | 5.9 |
| Local SEO: business types, several locations, opening hours, local blocks, KML sitemap | Yes, except the KML sitemap, which belongs to the main site | 5.9 |
| Knowledge Graph: site as a person or a company, social profiles | Yes | 5.9 |
| Analytics: Search Console and GA4 in the dashboard, index status, winners and losers, position history, email reports, AI search traffic, PageSpeed per post | Yes, added, opt-in. GA4 through the head code field. Our own analytics covers the rest | 5.5, 5.11 |
| Keyword rank tracker, Google Trends, AdSense | No. SerpDad, or not needed | 8 |
| Sitemaps: XML, news, video, image, exclude posts | Yes, with news, video and an HTML sitemap added | 5.2 |
| Redirections: 301, 302, 307, 410, 451, regex, debug, CSV import, automatic post redirects, 404 monitor with export | Yes, added | 5.2 |
| Image SEO: automatic alt and title, captions, find and replace, social image overlays, watermark | Yes, added, except watermarks | 5.1, 6 |
| Social: Open Graph, Twitter cards, default share image, previews, homepage tags | Yes, plus auto-generated share images | 5.2 |
| Site verification: Google, Bing, Yandex, Baidu, Pinterest, Norton | Yes, added | 5.2 |
| Advanced link options: nofollow external, new tab, cloaked links as external | Yes, added as the link policy, with `sponsored` for affiliate domains | 5.2 |
| Breadcrumbs | Yes | 5.2 |
| llms.txt generator, robots.txt editor | Yes | 5.2 |
| .htaccess editor | Not needed. The CMS is the server | |
| RSS optimisation: content before and after, source line | Yes, added | 5.2 |
| Role manager | Yes | 4.6 |
| Import from Yoast, AIOSEO, SEOPress and the Redirection plugin, CSV import of SEO data and focus keywords | Yes, added to the WordPress import | 5.7 |
| SEO Analysis tool, 30+ tests, score | Yes, as the site audit | 5.10 |
| Instant Indexing: IndexNow | Yes | 5.2 |
| Podcast module, Video SEO, News SEO | Schema and sitemaps yes. A podcast feed is Phase 8 | 5.2, 5.9 |
| WooCommerce, EDD, Elementor, Divi, bbPress, BuddyPress, AMP, Web Stories, custom post types | No. WordPress-only, or out of scope | 13 |
| Multisite | Yes | 4.4 |
| Version control, update notice | Yes, `khalid update` with rollback | 4.1 |
| Client management, white-label reports, agency features | Phase 8 | 10 |

### A.2 Ahrefs and Semrush, component by component

| Component | Ahrefs | Semrush | Lives in |
|---|---|---|---|
| Site crawl, issues by severity, health score, always-on audits, fix instructions, export | Site Audit | Site Audit | CMS, 5.10 |
| Core Web Vitals, lab and real-user | Site Audit | Core Web Vitals report | CMS, 5.10 |
| Internal linking: click depth, orphans, distribution, suggestions | Site Audit, internal linking suggestions | Internal Linking report | CMS, 5.10 |
| International SEO: hreflang checks | Localization | International SEO report | CMS, 5.4, 5.10 |
| Structured data validation | Structured data | Markup report | CMS, 5.9 |
| AI search readiness: llms.txt, semantic HTML, content freshness | | AI Search notices | CMS, 4.5, 5.2, 6 |
| Pages that get traffic but return 404 or noindex | Site Audit with Search Console | | CMS with Search Console, 5.11 |
| Crawl log: which bots fetched what and when | | Log File Analyzer | CMS, 5.5 |
| Web analytics | Web Analytics | Traffic Analytics | CMS, 5.5 |
| Alerts and scheduled reports | Alerts, Report Builder | My Reports | CMS, 5.5, 5.11 |
| Data export for Looker Studio | Looker Studio connector | Looker Studio connector | CMS, CSV and JSON export, 5.5 |
| Patches: fix on-page issues with a script | Patches | | Not needed. The CMS fixes at the source |
| Keyword research | Keywords Explorer | Keyword Magic Tool, Keyword Manager | SerpDad |
| Rank tracking | Rank Tracker | Position Tracking | SerpDad |
| Backlinks and link building | Site Explorer | Backlink Analytics, Backlink Audit, Link Building Tool | SerpDad, Ahrefs, Semrush |
| Competitor and market research | Site Explorer, Content Explorer | Domain Overview, Organic Research, Keyword Gap, Market Explorer | SerpDad |
| Content briefs and on-page scoring against the SERP | Content Helper, Content Grader | SEO Content Template, SEO Writing Assistant, On Page SEO Checker | SerpDad |
| AI visibility and brand monitoring | Brand Radar | AI Toolkit, Brand Monitoring | SerpDad |
| Local listings, social, advertising | | Listing Management, Social and Advertising toolkits | Not the CMS |

### A.3 The Ahrefs blog, page by page

Checked 2026-10-27 on ahrefs.com/blog: the home page, a post, a category page, an author page, the archive, the feed and the Japanese and Spanish editions.

| Ahrefs blog | In the CMS | Where |
|---|---|---|
| Byline with avatar and job title, "Reviewed by" | Yes, added | 4.12 |
| Date and reading time | Yes, reading time added | 4.12 |
| Linking websites count per post | No. Backlink data is SerpDad's | 4.12 |
| Sticky table of contents | Yes, added | 4.12 |
| Copy link on headings | Yes, added | 4.12 |
| Click to enlarge images | Yes, added | 4.12 |
| Glossary definitions on hover | Yes | 5.1, 5.13 |
| Author box, related posts, newsletter at the end | Yes, author box and related posts made default | 4.12, 5.6 |
| Cards with author, date and categories | Yes, added | 4.12 |
| Home page sections per category, topic links | Yes, added | 4.12 |
| Category and author pages with pagination | Yes | 4.11 |
| Team list, full archive | Yes, added | 4.12 |
| Grouped menu, search with a shortcut | Yes, shortcut added | 4.12, Phase 4 |
| Language editions, RSS | Yes | 5.2, 5.4 |
| Canonical, hreflang, Open Graph, Article schema, lazy images | Yes | 5.2, 5.9, 4.10 |
| Rotating promo banner, video and podcast cards | Announcement bar and video block yes; rotation and podcast cards no | 5.6, Phase 8 |

Sources: Rank Math pricing comparison, https://rankmath.com/pricing/ . Ahrefs Site Audit, https://ahrefs.com/site-audit and https://help.ahrefs.com/en/collections/1539899-issues . Semrush Site Audit issues list, https://www.semrush.com/kb/542-site-audit-issues-list .

---

## Appendix B. What Ghost and the other open-source CMSs have that the plan did not

Checked 2026-10-26 against the Ghost developer docs and help centre, including the 6.0 release and the 2026 changelog, and against WordPress, Strapi, Payload, Directus, Decap, Tina, Wagtail, Hugo, Zola, Hashnode and Quartz. Sources are at the end. Each item says who has it, why people love it, and what I recommend. D-11 and D-12 were decided yes on 2026-10-26. The v1 list and the three collections are folded into sections 4 to 11. The tables stay here as the record of where each idea came from.

### B.1 Add to v1

**A real public API and headless mode**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Content API: read everything with `filter`, `include`, `fields`, `formats`, `limit`, `page`, `order`, cached, safe for browsers | Ghost, WordPress, Hashnode, every headless CMS | Agencies render the blog inside their own Next.js or Astro site. That is a second answer to "how do I mount it at /blog": their app fetches the posts | v1. REST with an OpenAPI spec, Ghost's filter syntax, a TypeScript SDK |
| Admin API: write everything, with API keys per integration, staff tokens, `source=html` to create a post from HTML, and `updated_at` collision detection | Ghost, WordPress | Agents and scripts publish without the admin. Ghost's `source=html` is why so many tools target Ghost | v1. The SerpDad publish API becomes one client of it |
| Webhook events catalogue, 29 in Ghost: `post.published`, `post.scheduled`, `member.added` and so on | Ghost, Directus, WordPress plugins | Zapier, Slack and custom automations hang off named events | v1. Ghost's list, plus HMAC signing, which Ghost lacks |
| Headless starter kits | Hashnode, Ghost, Strapi | "Bring your own front end" in an afternoon | v1. One Astro and one Next.js starter that consume the Content API |

**Routing**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Collections with their own permalink patterns, such as `/guides/{slug}/` and `/news/{year}/{slug}/`, each with its own feed | Ghost `routes.yaml`, Hugo sections | Publishers shape URLs by content type | v1, as `routes.toml` |
| Channels: filtered lists with their own URL and feed, such as `/featured/` | Ghost | Curated pages with no extra work | v1 |
| Taxonomy URL prefixes, `/topic/` instead of `/tag/` | Ghost, Hugo, WordPress | URLs read the way the site talks | v1 |
| Aliases in front matter: old URLs that redirect to this post | Hugo | Migrations keep rankings with one line per post | v1, feeds the redirect manager |

**Editor and writing**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Bookmark card: paste a URL, get a preview card with title, image and description | Ghost | Links look like cards. Readers click them | v1 |
| Gallery, file download, product card with rating and button, toggle, divider, header card, HTML card with visibility rules | Ghost | The product card is the affiliate blogger's favourite. The file card is the lead magnet | v1. The product card writes Product schema |
| Snippets: save a block or a group of blocks and reuse it everywhere, update once | Ghost, WordPress patterns, Wagtail snippets | Affiliate disclosures, author boxes and CTAs stay in sync | v1 |
| Internal link search with `cmd+k` and `@` mentions of posts, tags and authors | Ghost | Linking becomes faster than not linking, which is exactly what the linter wants | v1 |
| Unsplash and GIF search inside the editor, basic image editing: crop, rotate, annotate | Ghost | Writers never leave the editor for an image | v1, Unsplash under its API terms |
| Emoji autocomplete with `:`, markdown shortcuts, slash menu | Ghost, Notion | Table stakes in 2026 | v1 |
| Autosave, unsaved-changes guard, version history with diff and restore in the admin | Payload, Wagtail, Strapi, Ghost post history | Nobody loses work. Git gives us the history; the admin needs the screen | v1 |
| Live preview pane in the real theme while editing | Payload, Strapi, Directus, Wagtail | See the real page, not an editor approximation. Our server renders in under a millisecond, so this is cheap | v1 |
| Post locking and presence: who else is editing this | WordPress | Two writers never overwrite each other | v1 |
| Multiple authors per post, custom excerpt, feature image caption, featured flag, custom template per post, per-network share cards for X and Facebook | Ghost post settings | Every one of these is asked for in the first week | v1 |
| Markdown directives for every block, so a plain text file can hold a bookmark card, a product card or an FAQ | Hugo shortcodes, Zola components, MDX | Agents and command-line writers can author anything the editor can. Without this, "CLI ready" is only half true | v1. One documented syntax, round-trips with the editor |
| Wikilinks `[[slug]]`, linked mentions at the bottom of a post, hover previews on internal links, a link graph view in the admin | Quartz, Obsidian Publish | Linking becomes visible and rewarding. Hover previews are a reader favourite, and they degrade to a plain link with scripts off | v1. The graph lives on the internal linking report |
| Maths rendered on the server as MathML | Quartz, Hugo | Technical blogs need it, and MathML needs no JavaScript | v1 |
| Co-located assets: a post folder with its images beside the markdown | Hugo page bundles, Zola | Writers and agents keep a post and its images together | v1, alongside the `media/` folder |
| An "Updates" list per post: dated notes of what changed | Wikipedia, good newsrooms | Shows freshness to readers and search engines, and matches the stale-post warning | v1 |

**Collaboration and workflow**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Review workflow: draft, in review, ready, published, with roles that can move a post between states | Decap, Wagtail, Strapi Review Workflows | Agencies and teams need an approval step. SerpDad's Writers already stage drafts for approval | v1 |
| Comments on drafts, with email notifications | Wagtail, Directus | Feedback lives on the post, not in Slack | v1 |
| Content calendar view | Every editorial tool | Scheduling by eye | v1, small |
| Git mode: each draft is a branch and a pull request, publish on merge, and the blog pulls from the repository on push | Decap, Tina, Hashnode GitHub sync | Developers write in their own editor and ship posts like code. Hashnode's GitHub sync is its most loved developer feature | v1, optional |
| Audit log of who changed what, trash with restore, bulk actions on the posts table | Wagtail, Directus, Strapi | Table stakes for teams | v1, small |

**Media**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Media library with folders, search, duplicate detection and a focal point editor that drives every crop | Wagtail, Directus | Crops stop cutting off faces | v1 |
| S3-compatible storage option, including Cloudflare R2 | Directus, Ghost adapters | Big media libraries do not live on the server's disk | v1 |

**Growth features**

| Feature | Who has it | Why people love it | Recommend |
|---|---|---|---|
| Form builder block: fields, spam protection, a submissions table, email and webhook delivery | Wagtail forms, WordPress plugins | Lead forms on blog posts are the whole point of a business blog | v1 |
| Subscriber segments and labels, several newsletters, open and click analytics, a welcome email and a short sequence after signup | Ghost, including its 2026 email sequences and dynamic member filters | The newsletter becomes a marketing channel, not a broadcast | v1 |
| Email-to-unlock gated content | Ghost members-only posts, lead magnet plugins | The oldest lead magnet there is, done in one block | v1 |
| Embeddable signup form for the main website | Ghost signup embed, Hashnode | The business site and the blog share one list | v1 |
| Announcement bar | Ghost | Promotions and launches without touching the theme | v1 |
| Navigation menus: primary, secondary and footer, nested, managed in the admin | Ghost, WordPress, Hugo menus | Editors change the menu without a developer | v1 |
| Design settings screen: logo, icon, cover, social accounts, fonts and colours from the synced tokens | Ghost design settings | Small changes without editing files | v1 |
| A logged-in toolbar on the live site: edit this post, lint status, this post's numbers | Ghost's 2026 admin toolbar, WordPress admin bar | The shortest path from reading your post to fixing it | v1 |
| Bluesky domain verification file served at `/.well-known/atproto-did` | Nobody does this for you | A free "verified" handle for the blog's domain | v1, trivial |
| oEmbed provider endpoint, so other sites embed your posts as cards | WordPress | Shares look good everywhere | v1, small |
| Scheduled unpublish, password-protected pages | Wagtail, Quartz | Promotions that end, drafts for a client | v1, small |
| `khalid theme check`: validate a template override before it goes live | Ghost gscan | Overrides stop breaking sites | v1, it is the linter on templates |

### B.2 Later, Phase 8

| Feature | Who has it | Why people love it | Why not v1 |
|---|---|---|---|
| Member accounts with passwordless login, paid tiers through Stripe, offers, portal, gift links | Ghost | Ghost's whole business. Creators love it | A different product. Business blogs sell elsewhere. Revisit if users ask |
| Social web: readers follow the blog from Mastodon, Bluesky and Threads, replies become comments | Ghost 6 ActivityPub | The 2025 and 2026 trend that made Ghost news again | Needs its own service and moderation. Start with auto-posting to Bluesky, Mastodon, LinkedIn and X |
| Cross-posting to Dev.to and Hashnode with a canonical back to the blog | Dev.to, Hashnode | Developer marketers reach three audiences from one post | APIs change often. Medium has closed its API |
| Mermaid diagrams | Quartz, Hugo | Technical blogs | Mermaid renders only in JavaScript. Needs a server-side renderer |
| Releases: group many changes and publish them together at one time | Strapi | Launch days | Nice to have |
| Visual editing on the live page, click to edit | Tina | Marketers love it | Big. The live preview pane covers most of it |
| Theme packaging, a theme directory, white-label admin, admin in other languages, client management for agencies | Ghost, Payload, Directus, Rank Math agency plan | Agencies | After the design settings prove themselves |
| Flows: visual automation inside the CMS | Directus | No-code people | Webhooks plus Zapier and Make cover it |
| GraphQL API | Hashnode, Strapi, Directus | Some front-end teams prefer it | REST plus the MCP server first |

### B.3 Skip

Recommendations and Explore networks, which read as a link network. Reader mode and stacked pages. Citations and canvases. Realtime APIs. AI writing in the CMS, which is SerpDad's job. Disqus. AMP. A dark-mode toggle in the admin: the admin is light only, and the public blog simply follows the main site's colours.

### B.4 Where the plan already beats Ghost

Worth saying on the free tools page. Ghost needs Node and MySQL 8 in production, Mailgun for bulk email, and Tinybird for its native analytics. Its hosted ActivityPub service caps interactions. It has no multi-site, no multilingual, no custom fields, no static mode, no built-in SEO linter or audit, and mounting it under a folder of another site is a support forum classic. The plan has one binary with SQLite, built-in email sending and analytics, multi-site, multilingual with hreflang, static mode, the linter, the audit, theme sync and the proxy rule generator.

Sources: Ghost Admin API, https://docs.ghost.org/admin-api/ . Ghost Content API, https://docs.ghost.org/content-api/ . Ghost webhooks, https://docs.ghost.org/webhooks/ . Ghost routing, https://docs.ghost.org/themes/routing/ . Ghost members, https://docs.ghost.org/members/ . Ghost cards, https://ghost.org/help/cards/ . Ghost post settings, https://ghost.org/help/post-settings/ . Ghost 6.0, https://ghost.org/changelog/6/ . Ghost changelog, https://ghost.org/changelog/ . Hugo content management, https://gohugo.io/content-management/ . Zola content, https://www.getzola.org/documentation/content/overview/ . Payload versions, https://payloadcms.com/docs/versions/overview . Strapi features, https://docs.strapi.io/cms/features . Directus features, https://directus.com/features . Decap editorial workflow, https://decapcms.org/docs/editorial-workflows/ . Tina docs, https://tina.io/docs/ . Wagtail features, https://wagtail.org/features/ . WordPress REST API, https://developer.wordpress.org/rest-api/reference/ . Hashnode, https://hashnode.com/ . Quartz features, https://quartz.jzhao.xyz/features/ .
