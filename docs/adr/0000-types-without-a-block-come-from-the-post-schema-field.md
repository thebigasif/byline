# Types without a block reach a post through its `schema` field

**Number:** next free. **Status:** Proposed, 2026-10-27. Gap audit 2, brief G15.

## Context

Plan section 5.9 lists schema for ItemList, Event, Course, Product, SoftwareApplication, Book, PodcastEpisode, ClaimReview and Dataset. ADR 0021 decided that these types, which have no editor block in Phase 1, come through schema templates per collection and a custom JSON-LD field, each checked against Google's required properties. `khalid-seo` can read custom JSON-LD (`CustomSchema`) and has a fixture for each of the nine types that `check_graph` passes (`tests/schema_fixtures.rs`).

Gap audit 2 found that none of this reaches a real page: post front matter has no field for custom JSON-LD, and `site.toml` has no schema templates yet (brief G11). The brief asks for typed builders for the nine types, or for presets documented in an ADR and each checked by `schema_check.rs`.

## Decision

- **A post's `schema` field holds its custom JSON-LD as text**, usually a YAML block (`schema: |`) with JSON inside, the way Google's examples are written. The build reads it with `CustomSchema::parse`, so it is escaped and checked like every other node, and adds its nodes to the page's graph. A field Khalid cannot read stops the build and names the file, what is wrong and how to fix it.
- **The presets are the nine fixtures** in `crates/khalid-seo/tests/fixtures/schema/`, one per type, each with every property Google requires. `schema_fixtures.rs` runs `check_graph` on each, so a preset that loses a required property fails Khalid's own tests. A writer copies the one they need into the field and changes the values.
- **No typed builders for the nine types yet.** A typed builder only pays off when an editor block or a setting fills it; until then it would be a second way to write the same JSON. When a block arrives, ADR 0021's rule holds: it gets its `SchemaBlock` variant and the fixture becomes its snapshot.
- **Templates per collection come with G11**, which reads schema settings from `site.toml`, and use the same presets.

## Consequences

- Every type in plan section 5.9 can now appear on a real page, built and served.
- The linter does not yet report a custom node's missing properties; brief G5 adds the page graph's gaps to its input. Until then the presets are the safe path.
- Custom JSON-LD is limited to 64 KB per post by `CustomSchema`.
