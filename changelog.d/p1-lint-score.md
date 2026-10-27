### Added

- `khalid lint` scores each post out of 100. Over a whole site it lists every post below 100 with its score, and `--json` adds a `scores` list with each post's score and the rules that cost it points. On one post it prints that post's score, and `--json` adds `score`. When the Admin API refuses to publish a post, its `lint` report now carries the same `score` (KH-LNT-4, plan D-10).
