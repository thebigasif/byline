# Khalid for AI agents

Khalid is a blog engine. This guide tells an agent, such as Claude Code, how to work on a Khalid blog safely. It is built into `khalid`, so it always matches the version you run.

## The site is a folder of text files

- `site.toml` holds the settings: the name, the address, the languages.
- `content/<language>/posts/` holds the published posts, one markdown file each, named `<date>-<slug>.md`.
- `content/<language>/authors/` holds one file per author. A post names its author by that file's slug.
- `drafts/<language>/<slug>.md` holds drafts. Nothing reads them but the Admin API, so they are never shown to readers.
- To take a page, glossary term, case study or changelog entry down without deleting it, move its file to the same place under `drafts/`, such as `drafts/en/pages/about.md`. Its webhooks then say unpublished, not deleted.
- `data/` and `public/` are made by Khalid from the files. Never write into them.

A post's identity is `<language>.<slug>`, such as `en.choosing-anchor-text`. It is the same for its draft and for the published post.

## Two ways in

**The command line.** Every command takes `--site <folder>` and `--json`. With `--json`, the result is one JSON value on stdout, and a failure is `{"error": {"what": ..., "why": ..., "fix": ...}}`. The exit code is 0 when it worked, 1 when something blocks and you must act, 2 when the command line was wrong, and 3 for a fault inside Khalid. Run `khalid --help` for every command. The ones you will use most:

- `khalid lint [post]` checks one post or the whole site. `--fix` makes the safe repairs.
- `khalid build` writes the site as HTML, and writes nothing when a dated post is blocked. With `mode = "static"` in `site.toml`, every form that would send to the site becomes a short note, and the output lists the features a static site has off, under `off` with `--json`.
- `khalid export`, on a site with `mode = "static"`, writes the whole site into `public/` and swaps it in only once every page is written, so pages of deleted posts are gone. Copy that folder to the host.
- `khalid audit` builds the site, crawls it the way Ahrefs and Semrush do, and scores its health out of 100. It exits with 1 on any error, names the heaviest page, and keeps the issues, the internal linking report, each page's weight and the score's trend in `data/audit/`. `--server http://127.0.0.1:8080` audits a running `khalid serve` instead of a build, and `--offline` leaves links to other sites unchecked.
- `khalid plan` suggests a day for each draft the linter passes, at the site's own pace or `--per-week <count>`, each topic group's pillar before its members, keeping scheduled days. It lists the drafts that wait and warns when the pace dropped. It changes nothing.
- `khalid serve` runs the blog and the APIs.
- `khalid watch` shows how fast and how reliably the blog answered at its public address over the last day and the last 30 days, from the looks `khalid serve` takes every few minutes.
- `khalid import wordpress|ghost|medium|substack|markdown <export> --dry-run` says what an import would bring in and leave behind. Without `--dry-run` it writes the posts with their old slugs and stores a redirect for each old address that moves. For `wordpress`, the export may be the site's address, which brings in its published posts and pages.
- `khalid keys create` makes a key. `khalid doctor` says what is wrong and how to fix it, and on a static site which features are off.
- `khalid backup --out <file>.tar.gz` writes one archive of the site folder with a whole copy of the database, safe while `khalid serve` runs. It leaves out `public/` and `data/media/`, which Khalid makes again, and never writes over a file.
- `khalid update` installs the newest release after checking it against the release's `SHA256SUMS`, and keeps the old version beside the binary. `khalid update --rollback` goes back to it.
- `khalid pull`, in git mode, brings the site's repository forward to what was pushed, checks the site, and builds a static site again. It never merges: when the blog's copy has its own changes, it stops and changes nothing.

**The MCP server.** `khalid --site <folder> mcp` speaks the Model Context Protocol on stdin and stdout. Each tool sends the request the HTTP API would get, so it obeys the same rules, keys and checks.

## Connect over MCP

1. Make a key once: `khalid --site <folder> keys create --scope posts:write --scope posts:publish --name Claude`. Leave out `posts:publish` for an agent that should only write drafts for a person to publish. The key is shown once, and Khalid keeps only a hash of it.
2. Give the key to the server in the `KHALID_KEY` environment variable, never on the command line and never in a file inside the site. In Claude Code: `claude mcp add khalid --env KHALID_KEY=<key> -- khalid --site <folder> mcp`.

The server checks the key when it starts. A missing, revoked or read-only key stops it with a message on stderr.

## The tools

Every operation of the Content API and the Admin API that a key may call is a tool. Its name is the operation's `operationId` in `/api/openapi.json`, its description is the operation's summary, and its arguments are the operation's parameters by name, with the request body as `body`. So a new route is a new tool the day it lands. Signing in, passkeys and passwords are for people in a browser, so they are not tools. The ones you will use most:

- `browse_posts` lists published posts, newest first. It takes `filter` (such as `tag:news+featured:true`), `order` (such as `published_at desc`), `limit` and `page`. Drafts are not listed.
- `read_admin_post` reads one post in any status by `id`, with its markdown and its `updated_at`.
- `create_admin_post` saves a new draft. Send `body` as `{"posts": [{"title": ..., "author": ..., "markdown": ...}]}`. To send HTML instead, which is cleaned and turned into markdown, put it in `html` and add `"source": "html"`. An `Idempotency-Key` makes a retry safe.
- `edit_admin_post` changes the fields of a post you send in `body` as `{"posts": [{...}]}`, with the `updated_at` you last read. To move a draft between review states, send `"settings": {"review_state": ...}` with `draft`, `in_review` or `ready`. Send `in_review` when a draft is finished, so an editor reads it. Only an editor, an admin or the owner may mark it `ready`.

Two tools are not operations of their own:

- `lint_post` runs the checks `publish_post` runs, without publishing and without changing anything. A draft is checked as it would go live today.
- `publish_post` publishes a draft, with its `id` and `updated_at`, once the linter lets it through.

Every answer is the API's own JSON. A tool that fails returns an error result holding the API's problem: a `detail` that says what happened and why, and a `fix` that says what to do.

## Writing a post that publishes

1. `create_admin_post` with a title, an author and the body.
2. `lint_post`. Each finding has a `rule`, a `severity` and `what`, `why` and `fix`. A finding with severity `block` stops publishing; `warn` does not.
3. Fix every blocking finding with `edit_admin_post`, using the `updated_at` from the last answer, and lint again.
4. `publish_post`. If it is refused, the answer lists every finding, and the post stays a draft.

Blocking rules include a post with no title, no author, no date, or no link to another post on the site. Link to other posts with site addresses, such as `/blog/why-internal-links-matter/`.

When an edit is refused with status 409, someone changed the post after you read it. Read it again with `read_admin_post`, make your change to the new copy, and send its new `updated_at`.

## Rules for agents

- Never print, log or save a key.
- Change content through the tools or the command line, not by writing into `data/` or `public/`.
- Leave publishing to `publish_post`, so the linter always decides what goes live.
