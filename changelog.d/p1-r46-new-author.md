### Added

- `khalid new --author "Sam Lee"` writes `author: sam-lee` into the post and makes `content/<language>/authors/sam-lee.md` when that author has no profile yet. Without `--author`, a new post names the language's only author, and when there is none or several, `new` says how to add one. `init` now suggests `--author` in its next step.
