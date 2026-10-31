# 0000. Deleted posts wait in `trash/`, and the key check keeps the audit log

**Number:** next free. **Status:** Proposed, 2026-10-31. Brief G52 (KH-EDT-6).

## Context

KH-EDT-6 asks for an audit log of who changed what and when, across content, settings and users, and a trash with restore. Content files are the source of truth (BL-FS-1), and a build reads only `content/`. Until now, deleting a post through the Admin API removed its file, so a slip of the finger lost the post for good. Every change to a site goes through the Admin API (CLAUDE.md: "the API is the only door"), and every Admin API route already passes through one key check, `require_admin_key`.

## Decision

- **A deleted post moves to `trash/<key>/` at the top of the site, under the path it had.** The key is the time of the delete to the second, then the post's identity, such as `trash/20261031T093000Z-en.hello/content/en/posts/hello.md`. A post kept as a folder moves as a whole folder. The move is one `rename`, so a post is never half deleted.
- **Restore moves the file back where it was, in the status it had.** A post that was live comes back live, so restoring it needs `posts:publish`, the scope deleting it needed. When another post took its address meanwhile, the restore is refused with a 409 and the copy stays in the trash; two posts never share an address.
- **Deleting for good removes the folder in the trash** and needs `posts:publish`. Nothing empties the trash on a timer: the founder's sites are small, and an automatic purge would delete without a person deciding.
- **The audit log is a visitor table, `audit_log`, written by the key check.** After a route answers a `POST`, `PUT`, `PATCH` or `DELETE` with a success, the key check records who (the key's or staff account's identity), what (added, changed or removed), the matched route and the address. Reads and refused writes are not logged. One rule covers every route, including routes added later, so the log cannot miss a new screen. The newest 100,000 rows per site are kept. Reading the log needs `admin:all`, which owners and admins have.

## Consequences

- `trash/` is outside `content/`, so a deleted post is never built or served, and a person can still find and copy the file by hand.
- Restoring a live post does not run the lint publish gate again. It passed the gate when it went live, and the restore puts back the same file. The schedule of a scheduled post is cleared on delete and is not restored; it comes back as a draft to schedule again.
- The audit log records the address of a change, not its content. It says that post `en.hello` was changed by key `1111222233334444` at 09:30, not which words changed; version history is a separate feature.
- A change made by editing files on the server, outside the API, is not in the log.
