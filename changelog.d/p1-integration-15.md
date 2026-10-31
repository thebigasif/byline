### Changed

- A blog's form answers, newsletter sign-ups and alerts now share one way of sending mail in `khalid serve`, so a site with `[mail]` set up gets all three from one setting.
- `khalid lint`, `khalid build` and the publish check now read the fact registry (`facts.toml`) and each post's brief (`briefs/`) together, so a post is held to both.
