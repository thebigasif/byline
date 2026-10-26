# Schema blocks are typed where an editor block exists

**Number:** next free. Renamed from 0015, which the theme track took on `main` first.

**Status:** Accepted, 2026-10-26. Phase 1 task 1.14.

## Context

Plan section 5.9 lists schema for many blocks: FAQ, how-to, video, review, recipe, listicle, event, course, product, software, book, podcast, fact-check and dataset. Task 1.7 builds only some of those blocks in Phase 1: FAQ, how-to, review, recipe, video and the business card. The others have no block yet, so there is no content to write their schema from.

## Decision

- `khalid-seo` has a typed input, a variant of `SchemaBlock`, for each block that 1.7 builds, and writes its node from that input.
- The types without a block in Phase 1 come through the schema templates per collection and the custom JSON-LD field, both checked against the schema.org vocabulary and Google's required and recommended properties. Each has a fixture there.
- When a later phase adds one of those blocks, it adds its `SchemaBlock` variant, and the fixture becomes the variant's snapshot.

## Consequences

- No typed input exists ahead of the block it would be filled from, so no field is guessed.
- Every type in section 5.9 can still be written and validated in Phase 1.
