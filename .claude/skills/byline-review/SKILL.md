---
name: byline-review
description: >-
  Review a pull request, diff, file or crate in the Byline repository against the
  rule catalogue and the 10-point rubric, and write a review where every finding
  cites a rule ID and shows the fix. Load this whenever asked to review, audit,
  grade, check or approve code in this repository, or before opening a pull request
  for a self-review. Pairs with rust-review, which holds the general Rust checks.
---

# Byline review

The goal is a review that finds nothing because the author already ran it. Scores are honest. A 10/10 with a hidden shortfall is the one unacceptable outcome.

## 1. Scope and tools

- Review exactly what was pointed at. For a pull request, the diff and what it touches.
- Run `just check` on the branch if a toolchain is available. If a hot path changed (`render`, `lint`, `server`, `content`), run `just bench` and compare with `perf/baseline.json`.
- Note the task from `docs/phases/` the change claims to complete, and its done line.

## 2. Check in this order

1. **Founder rules** in `CLAUDE.md`. Dark theme, placeholder text, an unapproved admin screen, a secret in a log or test, an em dash in any text, a user message without what, why and fix. Any of these is a must fix on its own.
2. **Correctness and soundness**: the `rust-review` list, plus: can this input escape the site folder, reach a private network address, inject HTML, or bypass a lint block?
3. **The rule catalogue**: walk `docs/standards/RULES.md` for every area the diff touches and cite IDs.
4. **Tests**: were they written first, do they cover failure modes, are they deterministic, do fixtures use real words, did a snapshot change without a reason?
5. **Budgets**: is a measure in `performance.md` at risk? Is the benchmark attached?
6. **Docs**: rustdoc, OpenAPI, CLI help, ADR, changelog line, plain language.
7. **Size and simplicity**: under 400 lines, one task, no speculative generality.

## 3. Write the review

~~~markdown
# Review: <pull request title>

<One paragraph: what the change does, which task it completes, whether `just check` and the benchmarks ran, the verdict, and the one most important thing to fix.>

## Rubric
| # | Point | Score | Note |
|---|---|---|---|
| 1 | Correct | 1 | |
| 2 | Tested | 0 | no test for the stale `updated_at` path |
| ... | | | |
**Total: 9/10**

## Must fix
### 1. <title> (`crates/byline-lint/src/rules/author.rs:42`)
**Rule:** BL-ERR-4
**Problem:** <what is wrong and the consequence, one or two sentences>
**Fix:**
```rust
<the smallest change>
```

## Should fix
## Consider
## What's already good
- <two to four specific things>
~~~

Rules for findings: every one has `file:line`, a rule ID or a BeautifulRust source rule, the consequence, and the fix as code. Group repeats. Respect documented intent. Keep it proportionate: five to ten findings for a normal pull request. Never pad.

## 4. Post it

Post as a pull request review with line comments where the findings sit. Request changes on any must fix. Approve only at 10/10, or at a lower score the author has explained and you agree with.
