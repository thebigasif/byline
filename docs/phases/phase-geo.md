# GEO phase: writing, freshness, structure, crawling and measuring

From the research of 28 October 2026. Decisions: `docs/decisions/2026-10-28-geo-build-list.md`. Every message, lint rule and report is in plain language and says what, why and how to fix. SEO, GEO and AEO behaviour comes only from Steve Toth and Koray Tugberk Gubur.

## First batch

### S1. Passkeys on webauthn-rs

- Status: `todo`
- Replace the hand-built WebAuthn code in `crates/khalid-auth/src/passkey.rs`, `passkey/` and `passkey_store.rs` with `webauthn-rs`. Keep Argon2 for passwords. `tower-sessions` for `session.rs` is optional.
- Done when: Passkey registration and sign-in pass with a soft authenticator, and no hand-built WebAuthn parsing remains.

### F1. An `updated` date on posts

- Status: `todo`
- Add `updated:` and an optional what-changed note to post front matter (fixes B1). Feed them to the visible date, `dateModified`, sitemap `lastmod` and IndexNow. The stale-post rule reads the updated date.
- Done when: A post with `updated:` shows it on the page, in JSON-LD, in the sitemap, and triggers IndexNow.

### P1. FAQ answers visible

- Status: `todo`
- FAQ and toggle blocks render their answers open by default, not collapsed in `<details>` (fixes B3).
- Done when: The FAQ block's answers are in the visible page with scripts off.

### P2. Scripts-off check in CI

- Status: `todo`
- Run `scripts/demo-check.sh` on every CI build (fixes B4). Waits for the workflow token scope (decision notes).
- Done when: CI fails when a page breaks with scripts off.

### P5. Author proof

- Status: `todo`
- Author `url` and `sameAs` in the post schema, a credentials field, and a site warning when there is no About or contact page (fixes B2).
- Done when: Post JSON-LD names the author with url and sameAs, and the audit warns without an About page.

### W1. Answer capsule

- Status: `todo`
- A question-style heading is followed by a standalone answer of 40 to 60 words, adjustable in `lint.toml` (D3).
- Done when: The rule fires on a short or missing answer and teaches the fix in plain words.

### W2. Paragraph length

- Status: `todo`
- Warn above about 225 words per paragraph; target 75 to 225, one idea each.
- Done when: Fixture with a long paragraph gets the warning.

### W3. Plain sentences

- Status: `todo`
- Flag hedge words, vague openers and trailing which-chains in answer paragraphs.
- Done when: Fixtures for each pattern fire with a plain fix.

### W4. Sourced statistics

- Status: `todo`
- Warn when a number has no linked source or no year near it.
- Done when: A bare statistic gets the warning; a sourced, dated one does not.

### A1. AI search audit checks

- Status: `todo`
- Replace the llms.txt-only AI search checks (open PR #663) with checks built on W1 to W4, P1 to P3 and F1. Keep llms.txt as low priority.
- Done when: `khalid audit` reports the new checks per page.

## Later, in this order

F2 (honest update switch), F3 (6-month review queue), P3 (AI info page), P4 (comparison, alternatives, best-of and review templates), P6 (optional brief per post), P7 (fact registry), P8 (topic group and main-post fields; fixes B6), P9 (page or section check), W5 (title and H1 agree, no near-duplicate headings), W6 (lists and tables where the heading calls for them), W7 (anchor text rules), C1 (visitor log, switched off until D5), C2 to C5 (first-crawl timer, crawl health, crawl alerts, bot error report; need C1), C6 (speed and uptime watch), C7 (page-weight meter), C8 (Bing Webmaster Tools and IndexNow on updates), M1 (AI visitors per tool), M2 (read but never cited), M3 (Search Console AI follow-up finder), M4 (publishing planner).
