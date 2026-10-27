# 0000. SEO settings: core reads their shape, `khalid-seo` checks their values at start

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 1, audit brief G11 (KH-SEO-1, KH-SEO-2, KH-SEO-4, KH-SEO-6, KH-SEO-7, KH-SEO-10).

## Context

`khalid-seo` already models every site-wide SEO choice: title templates, the default share image, the Twitter account, verification codes and files, crawler rules, feed options, the link policy, the publisher, local business locations and schema templates. None of them could be set, because `site.toml` only had `[seo.index]` and `[seo.indexnow]`. The rules for each value live in `khalid-seo`: which variables a template may use, what a Twitter handle or a verification code may hold, which file names are reserved. `khalid-core` reads `site.toml`, and `khalid-seo` depends on `khalid-core`, so core cannot call those rules.

Three ways were open: move every validated type into core, copy the rules into core, or let core read the shape and check the values one step later.

## Decision

- **Core reads the shape.** `SeoSettings` gains `titles`, `social`, `verification`, `crawlers`, `feeds`, `links`, `publisher`, `locations` and `schema`, as plain structs with `deny_unknown_fields`. Unknown keys, wrong types and words outside a fixed list (such as `nofollow`, `new-tab`, `mon`) are refused by the TOML reader with the line, as every other setting is.
- **`khalid` checks the values with `khalid-seo`'s own constructors** (`MetaTemplate::parse`, `TwitterHandle::try_new`, `RootFile::try_new` and so on) when it turns the settings into a `SeoSite`. Every command that loads `site.toml` makes that site once before it runs, so a wrong value stops the command at start, with the file, the line and the key, like a TOML mistake.
- **The line comes from the table.** A value's mistake names its table, such as `seo.social`, and its key. The message finds the table's header in `site.toml`, then the key below it, so `name` under `[seo.publisher]` is not confused with the site's `name`.

## Consequences

- Each rule exists once, in `khalid-seo`, and the Admin API settings screen can reuse the same check when it writes these tables.
- Core keeps a few small enums that mirror `khalid-seo`'s (`LinkRelSetting`, `LinkTargetSetting`, `FeedContentSetting`, `PublisherType`, `WeekdaySetting`). They are not `#[non_exhaustive]`, so the binary maps them with exhaustive matches and a new variant in core fails to compile until the binary handles it.
- A template or code that is wrong is reported by `khalid-seo`'s message, which quotes the value. The values are public (they appear in every page's head), so quoting them is safe.
