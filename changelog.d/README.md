# Changelog fragments

Each pull request that changes behaviour adds one file here instead of editing `CHANGELOG.md`, so parallel pull requests never conflict on it.

- Name: `<branch-slug>.md`, for example `p1-blocks-embed.md`.
- Content: the section on the first line (`Added`, `Changed`, `Fixed` or `Removed`), then the line for `CHANGELOG.md`, written for people who use Khalid.

The CTO folds the fragments into `CHANGELOG.md` under Unreleased when the pull request lands, and deletes them.
