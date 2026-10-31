### Added

- `khalid audit` builds the site into `data/audit/site/`, crawls it the way Ahrefs and Semrush do, and prints its health score out of 100 with each check that found something, its group, how many pages it found it on and the fix. It exits with 1 when it finds an error. `--json` prints the score, the counts and every issue (KH-LNT-5).
