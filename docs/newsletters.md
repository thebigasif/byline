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
