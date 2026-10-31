# 0000. A post without a focus keyword gets a warning

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 1, gaps brief G29 (KH-LNT-2, KH-LNT-4). Answers the founder question "Should a post without a focus keyword get a warning (audit brief B-LNT-4)?" in `docs/phases/phase-1.md`, under the founder's yes in advance of 2026-10-27.

## Context

The focus rules check a post's focus keyword in its search title, headline, description, first paragraph, subheadings, address and image descriptions. A post with no `focus_keyword:` gets none of these checks, so it can score 100 while nothing has checked its topic. The PRD asks for "a focus topic per post" (KH-LNT-4), and Rank Math shows no score at all until a keyword is set.

## Decision

A new rule, `no-focus-keyword`, warns on a post with no focus keyword, or one with no words in it. It is a warning, not a block, so it never stops a publish, and a site that does not want it sets `no-focus-keyword = "off"` under `[rules]` in `lint.toml`.

## Consequences

- The score always reflects the topic: a post without a keyword scores at most 95, and adding one turns on the checks that explain what to fix next.
- Posts imported from a blog without an SEO plugin start with this warning. The WordPress importer carries Yoast, Rank Math and AIOSEO keywords, so most imported posts keep theirs.
- The `clean` lint fixture now carries a focus keyword it satisfies, and fixtures built on it that change its text show the focus findings that change brings.
