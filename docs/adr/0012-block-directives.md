# 0012. Blocks beyond markdown are fenced directives

**Status:** Accepted, 2026-10-26. Phase 1 task 1.6, proposed by the coding agent and approved by the CTO.

## Context

Every editor block must also be writable in a text file, and parse then print must return the same text (KH-CON-5, BL-FS-5). Standard markdown covers headings, paragraphs, quotes, lists, code, dividers, tables and footnotes. The rest of the plan's blocks do not exist in markdown: callouts, figures with captions, FAQs, how-tos, product cards, forms and the others in plan section 4.6. They need one syntax that writers can type, agents can produce, and the editor can map to and from without loss.

## Decision

Standard markdown stays standard. Every other block is a fenced container directive:

```
:::callout{kind=tip}
Link from new posts to old ones, then go back the other way.
:::
```

- The opening line is `:::`, the directive's name, and optional attributes in braces: `name=value`, with the value in double quotes when it holds a space, `"`, `}` or `=`.
- The body, up to a closing `:::` line, is markdown, so a callout can hold a list or a link.
- A directive inside another uses more colons on the outer one (`::::faq` around `:::question`), the way fenced code nests.
- A directive Khalid does not know is refused with the file, the line and the name, never shown as plain text.
- Printing writes attributes in a fixed order per directive, so a block round-trips byte for byte in canonical form.

The first two directives are `callout` (`kind` is `note`, `tip` or `warning`; the body is the callout) and `figure` (`src` and `alt` are required, the body is the caption). A bare `![alt](src)` image stays valid markdown, and the linter suggests a figure when it has no caption.

## Why this syntax

`:::name{attr=value}` is what writers already know from remark-directive, the markdown directive proposal used across the unified and Astro ecosystems, and from MyST in the documentation world. Astro content collections and Tiptap's block model both map onto it directly: a Tiptap node's type is the name, its attributes are the braces, and its content is the body. Alternatives were refused:

| Alternative | Why not |
|---|---|
| Shortcodes, `{{< callout >}}` (Hugo) | Template syntax inside content; fragile when nested; Marmite's regex shortcodes are the cautionary example in our research survey |
| HTML elements, `<Callout>` (MDX) | Mixes markup into prose, and raw HTML in markdown is escaped by design (BL-SEC-1) |
| Fenced code with an info string, ` ```callout ` | The body would be code, not markdown, so it could not hold a link or a list |

## Consequences

- Each new block is one directive name, its attributes, a round-trip test and an HTML snapshot (BL-TST-5).
- The editor stores exactly what a writer would type, so a post moves between the editor and a text file without change.
- Writers coming from Astro, Docusaurus or MyST can bring their callouts with little or no change; the Phase 6 importers translate Hugo shortcodes and WordPress blocks into directives.
