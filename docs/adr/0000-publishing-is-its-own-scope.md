# 0000. Publishing is its own scope, held by editors and above

**Number:** next free. **Status:** Accepted, 2026-10-27. Audit 2, brief G6 (KH-EDT-4, KH-EDT-7).

## Context

Every Admin API post route needed `posts:write`, and a contributor's staff token held it, so a contributor could publish (audit finding F7). The plan's roles say a contributor writes drafts for others to publish. The brief asks to split `posts:publish` from `posts:write`, give it to editors and above, and answer a contributor's publish with a 403 in plain words. The permission table in `khalid-auth` (from #582) lets a writer publish their own posts, but the API does not know who wrote a post yet: a post records who last changed it, not who owns it.

## Decision

- **`posts:publish` is a new scope.** The owner, admins and editors hold it. Writers and contributors do not. An integration key holds it only when made with `--scope posts:publish`, and `admin:all` includes it.
- **An edit needs it when it changes what readers see:** it sets `status` to `published` or `scheduled`, sets `unpublish_at`, or touches a post that is already live or scheduled, which covers taking a post down. Any other edit of a draft needs only `posts:write`.
- **The 403 speaks to the person.** A staff member reads "A contributor cannot publish posts.", why, and "Save your changes as a draft and ask an editor to publish it." A key reads the usual message naming the scope. Both carry `rule: posts:publish`.
- **Writers wait for ownership.** Granting writers `posts:publish` would let them publish anyone's post, which is more than their role allows. Until a post records its owner and the API checks it, an editor publishes a writer's post.

## Consequences

- An integration key made with only `posts:write`, such as an older SerpDad key, now stages drafts and cannot publish them. Its owner makes a new key with both scopes. The changelog says so.
- The scheduler and `khalid build` are not API callers, so they are unchanged.
- When posts record their owner, `Role::Writer` gets a "publish own" check in the edit route, matching `khalid-auth`'s `PublishOwnPosts`.
