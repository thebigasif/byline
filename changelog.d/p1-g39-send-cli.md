### Added

- `khalid newsletter send <slug>` emails a published post to a newsletter's confirmed subscribers, once (task 5.8, KH-GRO-2). `--list` picks the newsletter and `--label` narrows it to a segment. Each copy holds the title, a link to the post, its text, the reader's own one-click unsubscribe link and the site's postal address. Khalid refuses to send until `[mail]` has a `postal_address`, and an edit never sends a post again. Migration 0020 keeps one row per send with its totals, never who got a copy.
