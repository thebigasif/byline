# Search Console follow-ups are long searches, matched to headings by topic words

**Number:** next free. **Status:** Proposed, 2026-10-31. GEO phase task M3, `docs/phases/phase-geo.md`. Built on G43 (`task/p1-g43-weekly`, PR #878), which is not on main yet.

## Context

M3 is named "Search Console AI follow-up finder" and nothing more. The research behind the GEO list is not in the repository, so this record fixes what the finder does.

The behaviour it points to comes from the two allowed sources. Koray Tugberk Gubur's query networks: a searcher's later, longer questions on a subject belong in the document that already ranks for it, so the page covers the context the searcher brings. Steve Toth's answer passages, which W1 already follows: a question-style heading with a short, standalone answer under it is the passage search and AI answers lift. Neither source says how to pick such questions out of Search Console or how to tell whether a page already answers one. Those are this project's choices, recorded here.

## Decision

- **A follow-up is a long search.** A search of at least 8 words (`FOLLOW_UP_WORDS`) that showed a published post in the last 28 days with final numbers. People ask AI assistants and Google's AI Mode in whole sentences, and those searches reach Search Console like any other. The word count is our choice, not either source's; it stays a constant until a site asks for another.
- **One request for the whole site.** `Session::searches` asks Search Analytics by page and query at once, up to Google's 25,000 rows, rather than one request per post.
- **A heading answers a search when it holds three in four of the search's topic words.** Topic words are the words left after filler and question words, each once; a word matches its plural `s`. No stemmer dependency, and the filler list is English: in another language every word counts, so the finder errs towards "no heading answers it yet".
- **Posts only.** A search that showed the home page or a list page is left out: those pages have no body to add a heading to.
- **The fix it teaches is W1's.** A heading that asks the question, with a 40 to 60 word answer under it (D3).
- **The text lists 10 searches a post; `--json` lists all.**

## Consequences

- Google leaves out searches too rare to share, and long searches are often rare, so a young or small site may see none. The command says so.
- The finder reads; it never edits a post. The writer decides which questions earn a heading.
- An admin screen and an Admin API route can call the same functions later; the admin screen needs a mockup first.
