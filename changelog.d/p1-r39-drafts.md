### Fixed

- `khalid publish` no longer says "Published" for a post it did not publish. A post in `content/` with no date counts as a draft, as the build already treats it, and publishing it dates it where it is. A post that is already live is refused with a message that says how to take it down (KH-LNT-1, KH-OPS-3).
- A refused `khalid build` now says to move a post to `drafts/<language>/` to keep it a draft.
