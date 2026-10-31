# Newsletters

Readers sign up with a plain form, confirm with one click in an email, and leave with one click in any email after that. Sending needs mail settings, see "Sending email" in `docs/deploy.md`.

## Several newsletters

Every site has one newsletter called `main`. To add another, make a folder for it in `content/newsletters/`, named in lowercase letters, digits and hyphens:

```text
content/
  newsletters/
    weekly/
```

Then name it in the sign-up block:

```markdown
::newsletter{label="Your email address" button="Get the weekly links" list="weekly"}
```

Leave `list` out and the form signs readers up for `main`. A form that names a newsletter with no folder saves nothing and tells the reader. Nothing in `content/newsletters/` becomes a page. One reader can get several newsletters, and unsubscribing stops all of them.

## Labels and segments

A label is a word you give a subscriber, such as `paid` or `from-launch`, in lowercase letters, digits and hyphens. A segment is one newsletter's confirmed subscribers who carry every label you name.

```sh
khalid --site <folder> newsletter label grace@example.com paid
khalid --site <folder> newsletter label grace@example.com paid --remove
khalid --site <folder> newsletter segment --list weekly --label paid
```

`segment` prints how many subscribers are in the segment. It never lists who they are. Add `--json` to either command for a program to read.

## Sending a post

A post is emailed only when you ask, and only once:

```sh
khalid --site <folder> newsletter send what-a-sitemap-does
khalid --site <folder> newsletter send what-a-sitemap-does --list weekly --label paid
```

The post must be published. Each confirmed subscriber in the newsletter, or in the segment the labels name, gets one copy: the title, a link to read it on the site, and its text. Forms, galleries and other blocks that only work on a page are left out. Every copy ends with that reader's own one-click unsubscribe link and the site's postal address.

Khalid sends nothing until `postal_address` is set under `[mail]` in `site.toml`, because anti-spam laws ask every newsletter email to show one. A post that went out before is never sent again, even after an edit. A copy the mail server refuses is counted and not sent again.
