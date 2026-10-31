# 0000. Every type in plan section 5.9 has a typed schema builder

Number: next free. Amends 0021.

**Status:** Proposed, 2026-10-31. Brief G15 in `docs/audits/2026-10-27-gaps-2.md`, KH-SEO-3.

## Context

ADR 0021 gave a typed input, a `SchemaBlock` variant, only to the blocks task 1.7 built: FAQ, how-to, video, review and recipe. Nine types from plan section 5.9 were left to the custom JSON-LD field and the schema templates, each proven by a JSON fixture: `ItemList`, `Event`, `Course`, `Product`, `SoftwareApplication`, `Book`, `PodcastEpisode`, `ClaimReview` and `Dataset`.

Brief G15 asks for typed builders for those nine, or an ADR that names them schema template presets. A template fills each property from one line of text, such as `{title}`. That cannot write what Google requires for most of them: an `Event`'s `location` is a `Place`, a `Product`'s price is an `Offer`, a `SoftwareApplication` needs both. A preset would publish a graph that the check then refuses.

Since 0021, the product card (`:::product`) landed in `khalid-content` with a name, a price, a score and a shop link, and nothing turned it into schema.

## Decision

- **Every one of the nine types gets a typed builder in `khalid-seo`,** a `SchemaBlock` variant with its own input struct, written the way the existing five are: an `@id` of `<page>#<type>-<n>`, properties in a fixed order, nothing written for a field left empty.
- **The constructor takes what the type cannot do without.** Where Google shows a rich result (`ItemList`, `Event`, `Product`, `SoftwareApplication`, `Dataset`), that is Google's required properties, so `check_graph` raises nothing on a graph a builder writes unless a choice is left open. `Product` is the one open choice: Google wants a price, a review or an average, any one of them, so a product with none blocks.
- **Types Google no longer shows as a rich result** (`Course` since 2025-09-09, `ClaimReview` since 2025, and `Book` and `PodcastEpisode`, which never had an open one) take what schema.org and Google's former guides named as their core: a `Course` its name, description and provider; a `ClaimReview` the claim, the verdict and the page; a `Book` its name and author; an episode its name and series. `check_graph` has no Google rule for them, and its tests prove it raises nothing for each.
- **Money is typed.** A price is a `Money` read from the text a writer types (`Money::parse("49.00", "GBP")`), so a price with three decimals or a lower-case currency is refused before it reaches a page.
- **The product card writes `Product` now,** with its price as an `Offer`, the writer's score as a `Review` by the post's author, and its picture and shop link as full addresses. A card with no price and no score writes no schema, because Google would ignore it and the check would hold the post back.
- **The other eight wait for their blocks.** No directive exists yet for an event, course, software, book, podcast, fact-check, dataset or listicle. Until the content track adds one, those types still come through the custom JSON-LD field, and the fixtures stay. When a directive lands, `khalid/src/build_schema.rs` maps it to the builder, as it does for the product card.

## Consequences

- An agent or a later block fills a struct and gets a graph Google accepts, with no JSON to hand-write.
- The builders are public API in `khalid-seo` before most have a caller in the binary. Their fields follow Google's guides, not a guess at a block, so a later directive maps onto them without changing them.
- The JSON fixtures under `crates/khalid-seo/tests/fixtures/schema/` keep proving that the custom field accepts the same types.
- A manual Rich Results Test run needs a browser and Google's site, which the build server does not have. Each pull request carries the JSON a builder writes, so the founder can paste it into the test.
