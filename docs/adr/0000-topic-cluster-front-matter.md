# 0000. A post names its topic cluster and its role in it

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, audit 2 brief G5 (KH-LNT-2).

## Context

The plan promises "pillar posts and their clusters" (section 4) and the linter has a `cluster-without-pillar` rule: a member of a topic cluster that does not link to the cluster's pillar post. The lint model has carried `cluster` with a name and a role since phase 1, but no post could say which cluster it is in, so the rule never fired on a real site (audit 2, G5).

## Decision

Post front matter gains one optional table, `cluster`:

```yaml
cluster:
  name: internal-links
  role: pillar
```

- `name` is a slug, so `Internal Links` and `internal-links` cannot name two clusters by accident. Every post in a cluster writes the same name.
- `role` is `pillar` or `member`, and `member` when left out, since most posts in a cluster are members.
- A role is only written inside the table, so a post cannot be a pillar of no cluster.

## Consequences

- The linter reads the field as it is and `cluster-without-pillar` fires through `khalid lint`, the Admin API and MCP.
- Nothing is rendered from the field yet. A theme or a list page can show a cluster later from the same field without a change to any post.
- An unknown role or a missing name is refused when the site is read, with the line and the roles that exist.
