# CTO answers, 2026-10-26

The founder has asked Claude, acting as CTO, to answer the questions the coding agent raises. This file is the record. Each answer names the question, the decision, the reason and what to do. Founder decisions in `docs/plan.md` sections 1 and 12 stay the founder's; everything else here is final unless the founder overrules it.

## 1. The name (task 0.1)

**Question.** Which name from `docs/decisions/name.md`? Byline is taken by an active open-source CMS.

**Decision.** The working name becomes **Wellranked**, as in "Wellranked by SerpDad". Byline, Typeset, Gazette, Masthead and the fallbacks Lede and Stylebook are out. Dateline, the first pick of the day, is withdrawn: the founder asked for a name that sounds like the solution to the customer's problem, and a dateline describes a newspaper, not an outcome.

**Reason.** The customer is the SEO or growth person who wants a blog that ranks without the WordPress mess. "A well-ranked blog" is the result they are buying, so the name says what the product does. On 2026-10-26 the name was free on GitHub, crates.io and npm, and the `.dev` and `.io` domains had no registration record; only `.com` is registered and serves an empty page. A web search found no company or tool called Wellranked. The runners-up, all free on the same registries, are Rankfast, Topranked, Ranksure, Rankbound and Outranked. The rejected names: Byline CMS is an active open-source headless CMS in our category; Typeset is a SamCart brand with a live US class 42 filing; Gazette is a common noun taken on every registry; Mastheads is a paid editorial platform next door to us; Lede is a publishing platform by Alley Interactive; Stylebook collides with the AP Stylebook, an editorial brand sold as software.

**Do.**
- Run the full checks from `docs/decisions/name.md` for Wellranked and the five runners-up, and add them to that file, so the record shows how the name was chosen.
- Rename the repository to `wellranked` and the crates to `wellranked`, `wellranked-core` and so on, in one pull request at the start of Phase 1, before more code depends on the name. GitHub redirects the old address. Update `CLAUDE.md`, the plan and the standards in the same pull request.
- Buy nothing and publish nothing under the name yet. The founder decides on domains after a trademark lawyer confirms Wellranked for classes 9 and 42 in the US and EU. The lawyer's check is a launch blocker, not a Phase 1 blocker.
- The founder may veto this pick. Until then it stands.

## 2. Accented slugs (task 0.2, from pull requests #6 and #7)

**Question.** Should slugs allow accented letters such as `ü`, so `über-seo` is not forced to `uber-seo`?

**Decision.** Slugs stay ASCII by default, with language-aware transliteration. Unicode slugs are a per-site opt-in.

**Reason.** ASCII slugs are identical on every server, every shell and every clipboard, and that is what most of our users expect. But forcing `uber` for `über` is wrong for German readers; the German convention is `ueber`. And for scripts where ASCII is unreadable, such as Arabic, Japanese or Cyrillic, Google indexes UTF-8 URLs fine and a transliterated slug helps nobody.

**Do.**
- `Slug::from_title(title, locale)` transliterates by locale: German maps the letters ä, ö, ü and ß to the pairs ae, oe, u plus e, and ss; other Latin scripts strip diacritics through Unicode decomposition; everything else falls back to the Unicode path below.
- `site.toml` gets `slugs = "ascii" | "unicode"`, default `ascii`. In `unicode` mode a slug is NFC-normalised lowercase letters, digits and hyphens from any script, stored as written, percent-encoded only where a URL needs it, and the 200-byte limit still applies to the encoded form.
- The linter warns on long slugs and on a Unicode slug in an `ascii` site, with the fix being the transliterated form. Phase 1 scope, under the content model task. The current `Slug::try_new` stays as the strict ASCII constructor.

## 3. The release test tag (task 0.7, pull request #16)

**Question.** May a `v0.0.1` tag be pushed to test the release workflow?

**Decision.** Yes, as a prerelease tag, once #16 is on `main`.

**Do.** Push `v0.0.1-alpha.1` on `main`. The workflow must produce a draft release with four binaries and `SHA256SUMS`. Verify the four files download and the checksums match, write the result in the pull request or the phase brief, then delete the draft release and the tag. Real releases keep the plan's rule: CI builds from a tag, the founder presses publish.

## 4. Standing answers for questions not yet asked

- **Merging.** Pull requests are merged by the CTO session after the required checks pass. The coding agent never merges. It keeps stacking small pull requests and retargets a child to `main` when its parent lands.
- **Stacked branches.** A pull request's base branch is deleted only after every child has been retargeted. GitHub closes a child whose base disappears.
- **Anything in the plan's section 12.** Decided as written there. Do not reopen.
- **Phase 1.** Starts when the founder says go. The first Phase 1 pull request is the rename above.
