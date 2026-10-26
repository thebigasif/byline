# 0004. Errors reach people as what, why, fix

**Status:** Accepted, 2026-10-26. From the founder's plan, section 6 and decision 5.

## Context

The founder's thesis is that the free tool teaches SEO. A lint block that says `E042: author missing` teaches nothing. Users who learn become SerpDad customers. The same applies to every CLI error, API error and locked feature card.

## Decision

Every message a person can see is a `UserMessage` with three fields: `what` happened, `why` it matters, what to do to `fix` it. Each is one short sentence in plain words. Library crates return typed errors with their causes (BL-ERR-1, BL-ERR-2); the binary and the API map them to `UserMessage` at the edge (BL-ERR-4). Lint messages carry a `rule` and a link to a SerpDad guide. API errors are RFC 9457 problem details with the same three parts (BL-ERR-5).

## Consequences

- One test walks every message in the catalogue and checks the three parts, their length and the absence of jargon and em dashes.
- Messages are written once, in a strings file, and reused by the CLI, the API and the admin.
- A message without a fix is a bug.
