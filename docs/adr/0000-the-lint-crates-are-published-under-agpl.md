# 0000. The lint crates are published under the AGPL, and SerpDad uses them as the copyright holder

**Number:** next free. **Status:** Accepted, 2026-10-31. Audit 2, brief G31 (KH-LNT-6). The founder said yes to everything in advance on 2026-10-27; this records the choice taken under that yes.

## Context

KH-LNT-6 asks for the lint engine as a crate SerpDad's backend can depend on, published with no AGPL-incompatible dependency. Every crate inherited `publish = false`, version `0.0.0` and `AGPL-3.0-only` from the workspace, so nothing could be published, and the audit asked which licence the published crates should carry while the product is AGPL.

The inputs are already decided. Plan decision 9: Khalid is AGPL-3.0, so nobody can take it closed. Plan section 3, "Licensing of shared code": SerpDad owns the copyright, so it may use its own code under any licence in the paid product, and the CLA (`CLA.md`) gives it the same right over outside contributions. ADR 0005 keeps every dependency permissive for the same reason. The PRD's line "Permissive dependencies, AGPL product. So SerpDad can reuse the crates and nobody can sell a closed fork." asks for exactly that pair.

Three choices were weighed:

1. **MIT or Apache-2.0 for the two crates.** SerpDad could depend on them without thinking about copyright. But the linter is one of the things that makes Khalid different, and a permissive licence would let anyone ship it inside a closed competitor. That reopens decision 9 for the most valuable part of the code.
2. **Dual licence, AGPL or a commercial licence.** That is what copyright ownership plus the CLA already gives SerpDad, with a second licence text to keep and nobody else to sell it to.
3. **AGPL-3.0-only, published as is.** Everyone else gets the product's terms. SerpDad, as the copyright holder with a CLA on every outside change, is not bound by them.

## Decision

- **`khalid-core` and `khalid-lint` are published to crates.io under `AGPL-3.0-only`,** the licence the workspace sets. The licence stays inherited from the workspace, so the two crates can never drift from the product. A test, `khalid-lint/tests/publishable.rs`, fails if either one does.
- **SerpDad depends on them as the copyright holder.** Its backend may link the crates.io copies. Its own `cargo deny` names these two crates as an exception, with this ADR as the reason, the way ours names MPL crates.
- **The CLA is what makes that safe.** An outside change to either crate lands only after its author signs the CLA, which `.github/workflows/cla.yml` already enforces on every pull request.
- **Both crates start at `0.1.0` and always share one version.** `khalid-lint`'s public API carries `khalid-core` types, so a change in one is released with the other. The workspace names them once in `[workspace.dependencies]`, and every member uses `khalid-core.workspace = true`, so a release bumps one line per crate. `0.x` says the API may still change; `1.0.0` comes with the product's 1.0 (plan Phase 7).
- **Only these two are published.** Every other crate keeps `publish = false`. `khalid-lint` depends on nothing else of ours, and a test-only self dependency in `khalid-core` is path-only, so `cargo publish` leaves it out.
- **The check.** `just publish-check` and a step in `scripts/check.sh` run `cargo publish --dry-run` for both crates, which packages them and builds each from its package against crates.io, as a real upload would. `deny.toml` gives the two crates a named `AGPL-3.0-only` exception now that they are no longer private; their dependencies still meet the permissive allow-list.
- **Uploading is the founder's.** Nothing here uploads. A real `cargo publish` needs a crates.io token, which agents never handle; the founder runs it, or adds it to the release workflow with the token entered as a secret.

## Consequences

- A company that bans the AGPL cannot use the crates. That is the same cost the plan already accepts for the product, and the README FAQ answers it.
- A pull request from another track that adds a member with `khalid-core = { path = "../khalid-core", version = "0.0.0" }` stops resolving once this lands. The fix is `khalid-core.workspace = true`.
- CI does not run the dry run yet: `.github/workflows/ci.yml` belongs to the CTO, and its test step runs without network. The offline manifest test runs there on every pull request; the CTO may add `just publish-check` to the Rust job.
