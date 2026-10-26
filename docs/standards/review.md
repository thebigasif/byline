# Review: what 10/10 means

Every pull request is reviewed by an agent before a person sees it, and the goal is a review that finds nothing. This rubric is how we get there. The author scores it first and pastes the scores in the pull request. The reviewer scores it again. Each point is 0 or 1.

| # | Point | 1 means |
|---|---|---|
| 1 | Correct | The behaviour matches the task's done line, and the failure modes are handled. A reviewer cannot construct an input that breaks it |
| 2 | Tested | Tests exist for the behaviour and each failure mode, were written first, are deterministic, and would fail if the code were wrong |
| 3 | Errors | Every error keeps its cause, reaches the person as what, why and fix, and no path can panic |
| 4 | Types | Invalid states are unrepresentable: enums for modes, newtypes for identifiers, ordered collections for output |
| 5 | API and naming | Follows RULES.md BL-API-*: borrowed parameter types, no `bool` parameters, names per the API Guidelines, `#[non_exhaustive]` where it should be, OpenAPI entry present |
| 6 | Safe and secure | No `unsafe`, inputs validated at the edge, paths through `SitePath`, HTML sanitised, outbound through `Outbound`, nothing secret logged |
| 7 | Fast | Inside every budget, benchmarks attached for hot-path changes, nothing computed at request time that belongs at publish time |
| 8 | Documented | Public items documented, crate docs say what the crate owns, an ADR for any new decision, plain language everywhere |
| 9 | Simple | No speculative generality, no dead code, functions and files inside the size rules, the smallest change that does the job |
| 10 | Consistent | Zero suppressions without reasons, no new dependency without its sentence, follows the patterns in `rust.md`, no founder rule touched |

A pull request below 10 can still merge when the author says which point is below and why, and the reviewer agrees. A pull request that hides a shortfall does not merge.

## How to run a review

1. Load the `rust-review` skill and the `khalid-review` skill.
2. Run `just check` on the branch. Fold the results in. Never paste raw tool output.
3. Read the diff and everything it touches. For every finding: `file:line`, the rule ID from RULES.md or the source rule from BeautifulRust, the consequence, the smallest fix as code.
4. Sort: must fix, should fix, consider. Group repeats.
5. Score the rubric. Say what is good. A review with no good points is not credible.
6. Post the review as a pull request review. Findings go on the lines they concern.

## What reviewers are not for

Style the formatter owns. Lints clippy owns. Decisions the founder owns. A reviewer who wants a founder decision changed opens an issue, not a finding.
