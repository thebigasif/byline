# Getting started with Khalid

This page takes you from nothing to a published post, then shows how to check the site, sign in to the admin, bring in an old blog, and back up and restore. Every command on this page is run by a test (`crates/khalid/tests/getting_started.rs`), so it works as written.

A few words first:

- A **site folder** holds one blog: its settings in `site.toml`, its posts as text files, and its database in `data/`.
- **Front matter** is the block between the two `---` lines at the top of a post. It holds the title, the author and other facts about the post.
- A **slug** is the short name of a post in its address, such as `why-internal-links-matter`.
- A **draft** is a post that is not live yet. Drafts live in the `drafts/` folder of the site.
- The **linter** is the checker that reads every post before it goes live. Some of its findings only warn; others block publishing until you fix them.

## 1. Get Khalid

Khalid has not had its first release yet. Until it has, build it from source as [CONTRIBUTING.md](../CONTRIBUTING.md#build-khalid-from-source) says. Building the admin needs Node.js; without it you get a working blog but no admin screens.

Once the first release is out, one line installs it on Linux or a Mac:

```text
curl -fsSL https://raw.githubusercontent.com/thebigasif/khalid/main/scripts/install.sh | sh
```

Check that it runs with `khalid --version`.

## 2. Make a site

Pick a folder name, here `blog`, and the address the blog will have on the internet. You can change the address later in `site.toml`.

```sh
khalid init blog --address https://blog.example.com --yes
```

Leave out `--yes` to answer the questions one by one: the address, the folder of the website the blog lives under (such as `/blog/`), the languages and whether Khalid runs as a server or writes plain files.

## 3. Say who writes

Every post needs an author, and every author needs a profile page. Save this as `blog/content/en/authors/sam.md`:

```markdown
---
name: Sam Rivera
slug: sam
role: Editor
---

Sam writes about running a small business website.
```

`slug` is what posts put after `author:`. The text under the front matter appears on the author's page.

## 4. Write your first post as a draft

A draft goes in `drafts/<language>/<slug>.md`, with no `date:` line. Save this as `blog/drafts/en/why-internal-links-matter.md`:

```markdown
---
title: Why internal links matter
slug: why-internal-links-matter
author: sam
description: Internal links help readers and search engines find the rest of your blog. Here is how to add them.
---

An internal link is a link from one page of your site to another. Readers follow them to read more, and search engines follow them to find and rank your pages.

## Link from every post

The linter blocks a post that links to no other page of your site. Your first post has nothing else to link to yet, so link to [the blog's home page](/). That counts.

## Use words that say where the link goes

"Read our guide to title tags" tells a reader what they get. "Click here" does not.
```

A post is plain markdown: `##` makes a heading, `[words](/address/)` makes a link, and `![description](picture.jpg)` adds a picture.

`khalid --site blog new --draft "Your title" --author "Sam Rivera"` makes the same kind of draft for you, with the `author:` line, and makes the author's profile when there is none. Without `--draft`, `new` writes a dated post straight into `content/en/posts/`, so it is live at the next build.

## 5. Publish it

```sh
khalid --site blog publish why-internal-links-matter
```

Khalid checks the draft first. If a rule blocks it, Khalid says what is wrong, why it matters and what to do, and publishes nothing. Otherwise it moves the file to `content/en/posts/`, adds today's date, and tells any search engines and webhooks you have set up.

## 6. See the blog

```sh
khalid --site blog serve
```

The blog is now at `http://127.0.0.1:8080/`. Leave it running while you write: each saved change shows at once. Press Ctrl+C to stop it. `docs/deploy.md` shows how to run it on a server for good.

## 7. Sign in to the admin

The admin is the web editor at `/admin/`. Make yourself its owner:

```sh
khalid --site blog staff owner --email you@example.com --name "Sam Rivera"
```

Khalid prints a sign-in link once. It works for 15 minutes, and only once. It shows the link twice: on the address in `site.toml`, and on `localhost` for this computer. While you try Khalid on your own computer, open the `localhost` link while `khalid serve` runs. Once signed in, choose a password or add a passkey. To sign in again later, run the same command.

If `/admin/` says the admin is not built, your copy of Khalid was built without it: see [CONTRIBUTING.md](../CONTRIBUTING.md#build-khalid-from-source).

## 8. Check the site

```sh
khalid --site blog lint
khalid --site blog build
khalid --site blog audit
```

- `lint` checks every post and scores it out of 100. Warnings do not stop anything, but each one says how to do better.
- `build` writes every page as plain HTML into `blog/public/`, the way static hosting needs it. It writes nothing if a dated post is blocked.
- `audit` builds the site and crawls it the way Ahrefs and Semrush do, then scores its health out of 100. It keeps its reports in `blog/data/audit/` and exits with an error when it finds one. Add `--server http://127.0.0.1:8080` to crawl the running blog instead.

Each linter message ends with a rule and a guide, such as "Rule no-internal-link, guide internal-links". The rule is the check that fired. The guide names a SerpDad guide that explains it at length; those guides are not published yet, so until then the three sentences in the message are the whole advice. Every message is listed in `crates/khalid-lint/catalogue.toml`, and `khalid --site blog lint --json` gives them in a form scripts can read.

Six rules block publishing: no title, no date, no author, no link to another page of the site, more than one main headline, and structured data that misses a field Google needs.

## 9. Bring in an old blog

Khalid imports from WordPress, Ghost, Medium, Substack and folders of markdown. It keeps every slug and stores a redirect for each address that moves. Try it with the sample WordPress export in Khalid's source, at `crates/khalid-import/fixtures/wordpress/basic.xml`, copied beside your site folder as `export.xml`:

```sh
khalid --site blog import wordpress export.xml --dry-run
```

`--dry-run` says what would come in and what would stay behind, and writes nothing. Run it again without `--dry-run` to bring the posts in. Imported posts keep their dates, so each must pass the linter before `build` writes the site.

## 10. Back up and restore

```sh
khalid --site blog backup --out blog.tar.gz
khalid restore blog.tar.gz blog-restored
```

`backup` writes one archive of the site folder with a whole copy of the database, and is safe while `serve` runs. It leaves out `public/` and `data/media/`, which Khalid makes again.

`restore` makes the site again in a new or empty folder, database included. It never unpacks over a folder that holds files, so it cannot mix an old copy into a live site. To put a backup in place of a live site:

1. Stop `khalid serve`.
2. Move the site folder away, such as `mv blog blog-old`.
3. Run `khalid restore blog.tar.gz blog`.
4. Start `khalid --site blog serve` again, and delete `blog-old` once the blog looks right.

## Where next

- `docs/deploy.md`: run Khalid as a service, in Docker, or on Fly.io, Railway, Coolify or a Hetzner server.
- `docs/newsletters.md`: send posts to subscribers by email.
- `khalid --help` lists every command, and `khalid <command> --help` explains one.
- `khalid agents` prints the guide for AI agents that write and publish through Khalid.
