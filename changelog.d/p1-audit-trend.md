### Added

- `khalid audit` keeps every issue in `data/audit/issues.csv`, the internal linking report in `data/audit/links.csv`, and each run's score and counts as one line of `data/audit/runs.jsonl`. It ends its report with the last scores, oldest first, and `--json` adds the whole trend and the three files' paths (KH-LNT-5).
