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
