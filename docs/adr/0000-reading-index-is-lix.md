# 0000. The hard reading warning measures LIX

**Number:** next free. **Status:** Proposed, 2026-10-26. Phase 1, audit brief B-LNT-2 (KH-LNT-2).

## Context

Plan section 6 asks for a warning when a post's "reading grade" is above a threshold, and the audit asks for sentence and word length per language. The common grades, Flesch Reading Ease and Flesch-Kincaid, count syllables. Syllables differ by language and need rules or a dictionary per language, and German has its own Flesch variant (Amstad) with other weights. The linter must stay small and depend on `khalid-core` only (ADR 0011).

## Decision

`hard-to-read` measures LIX, the Swedish readability index: the average words per sentence, plus the share of words longer than six letters in percent. One formula serves every language, and both of its parts are exactly the sentence and word length the audit names. Below 40 reads as easy, 50 as a newspaper, 60 or more as very hard, in English and German alike.

The language-specific part is where a sentence ends: a short form with a full stop, such as "e.g." or "z. B.", does not end one. A line break does, because the binary's plain text ends a line at every block. Posts under 100 words are not measured, because the index means little on a few sentences. The limit is `hard-reading-index` in `lint.toml`, default 60.

## Consequences

- No syllable rules and no new dependency. A new language needs only its short forms.
- LIX scores a text with many long compound words as harder, which fits German readers too: long compounds are what makes German hard to read.
- A site that writes for specialists can raise `hard-reading-index` in `lint.toml`.
- If the founder later wants a grade in school years, as Flesch-Kincaid gives, it can be shown next to LIX without changing the rule.
