### Added

- The site database keeps the analytics: page views in `analytics_counts`, and a new `crawl_log` table with which bots asked for which address each day, the latest time and the status they got. Both are visitor data that a rebuild keeps. The crawl log holds 10,000 rows a day and keeps 90 days (KH-GRO-4).
