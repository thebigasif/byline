### Changed

- `khalid build` refuses to publish a post the linter blocks. It checks every dated post first, and if any is blocked it writes no page at all, lists each problem in the same plain sentences as `khalid lint`, and exits with 1. `--json` lists the refused posts' findings. A post with no `date:` is a draft and is not checked, and `lint.toml` turns rules into warnings or off as it does for `khalid lint` (KH-LNT-1, KH-LNT-3).
