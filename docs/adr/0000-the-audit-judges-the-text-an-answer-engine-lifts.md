# 0000. The audit judges the text an answer engine lifts, the way the linter does

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase GEO, task A1 (KH-LNT-5, decisions of 2026-10-28).

## Context

Task A1 replaces the llms.txt-only AI search checks with checks built on W1 to W4, P1 and F1, reported per page by `khalid audit`. W1 to W4 already exist as lint rules that read a post's source. The audit must not know about content files: it reads pages as a crawler does, from a built folder or a running server. So the same rules have to be read again from HTML, and the two readings must not disagree about one post.

## Decision

- Every audit check carries a reason beside its name and its fix, so each issue reads in three parts: what is wrong, why it matters, how to fix it.
- The page reader keeps the page's own text as an outline of subheadings and paragraphs. It finds that text from semantic HTML, not from Khalid's templates: it leaves out `header`, `nav`, `aside`, `footer` and `form`, and on a page with an `<article>` it reads only the outermost one, so comments do not count. A link to a place in the page is left out of a heading's text, so a heading link does not hide the question mark.
- The writing checks run on pages whose `og:type` is `article`, and on pages that give no `og:type`, such as a site Khalid did not build. Lists, profiles and the home page hold counts and links, not answers.
- The answer under a question heading is every paragraph up to the next heading, counted the way the linter's W1 rule counts it, so a post the linter passes is not faulted by the audit. The 40 to 60 words (decision D3) and the 225-word paragraph (W2) are the defaults; `Thresholds` can change them.
- The llms.txt checks stay, as notices, because llms.txt is never the main AI search feature.

## Consequences

- One rule gives one answer in both tools. A stricter reading, the first paragraph alone, would fault posts the linter publishes.
- A theme that marks up its text without semantic elements gets every paragraph checked, menus included. That is the price of not knowing about templates.
- The word lists for plain answers (W3) are copied from the linter's, because the linter keeps them inside its crate. They live in two places until a shared home exists, and a change to one must be made to the other.
