### Changed

- A site's first post can follow every lint message: the "no internal link" fix now says a link to the home page counts while it is the only post, and a language's only post is no longer reported as an orphan.

### Added

- The linter warns when a post's `author:` has no profile in `content/<language>/authors/`, the gap that left bylines linking to an author page that was never built.
