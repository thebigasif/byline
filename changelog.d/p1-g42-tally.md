### Added

- `khalid-analytics` keeps a tally of views and bot visits in memory between writes, so counting a page costs no database write, and defines where the counts are stored. One tally holds at most 10,000 rows, and the crawl log at most 10,000 rows a day for 90 days, so made-up addresses cannot fill memory or the disk (KH-GRO-4).
