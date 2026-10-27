### Added

- `[seo.crawlers]` in `site.toml` shapes `robots.txt`: paths no crawler may read, and crawlers allowed or blocked by name, such as `GPTBot` (audit G11).
- `[seo.feeds]` sets what every feed carries: whole posts or excerpts, text before and after each item, a line naming the original post, and how many posts it holds.
- `[seo.links]` sets the site's link policy: `nofollow`, `sponsored` or `ugc` on external links, new tabs, and affiliate and internal domains. Bodies follow it once audit G17 lands.
