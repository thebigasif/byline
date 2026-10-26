# 0027. Maths is drawn with math-core, not latex2mathml

**Status:** Accepted, 2026-10-26. Phase 1 task 1.8.

## Context

Task 1.8 draws maths on the server as MathML, so formulas show with scripts off. `docs/standards/rust.md` and the phase brief name `latex2mathml`. Its last release, 0.2.3, is from 2021, and reading its source showed three problems for a blog that renders what writers type:

- **It escapes nothing.** `a < b` becomes `<mo><</mo>`, which is broken HTML, and text inside `\text{}` goes into the page as written. Every page body has to be safe HTML (BL-SEC-1).
- **An unknown command is drawn as text,** `[PARSE ERROR: ...]`, into the reader's page, instead of being reported to the writer.
- **Some output is invalid,** such as the closing tag `</mro>` after a sized bracket.

`math-core` 0.9 (MIT, maintained, Rust 1.96) converts LaTeX to MathML Core, the part of MathML that Chrome, Firefox and Safari draw natively. It escapes text, returns an error for an unknown command or an unclosed group, and its dependencies pass `cargo deny`.

## Decision

- **`khalid-content` uses `math-core`** with its defaults: no pretty printing, unknown commands are errors, and one converter is built once and shared (BL-PERF-2).
- **`$...$` is inline maths and `$$...$$` is a block,** through comrak's `math_dollars` option. A dollar sign followed by a digit or a space, as in a price, stays text. `\$` writes a dollar sign anywhere.
- **`khalid --site` refuses a formula that cannot be drawn,** with the file, the line and `math-core`'s plain label, such as "a group was never closed". A formula that still fails at render shows its source as escaped text.

## Consequences

- The `latex2mathml` row in `docs/standards/rust.md` now names `math-core`.
- MathML Core leaves out a few presentational features of full MathML, so a rare formula may look plainer than in LaTeX. `math-core` refuses the commands browsers draw unreliably rather than drawing them badly.
- Equation numbering and `\label` references across one post are possible later with `math-core`'s global state, without changing the markdown.
