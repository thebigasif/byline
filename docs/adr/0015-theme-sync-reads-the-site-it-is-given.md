# 0015. Theme sync reads the site it is given, and only that site

**Status:** Accepted, 2026-10-26. Phase 3 work pulled forward, task 3.1.

## Context

Plan section 4.9 says theme sync fetches the home page and its stylesheets. It does not say which stylesheets, how much to read, or what to do when one of them fails. A home page can link a stylesheet anywhere: its own server, a CDN subdomain, a font service, an ad network. Every fetch goes through `Outbound` (RULES.md BL-RUN-8), which already refuses private addresses and caps one response at 10 MiB.

## Decision

- **Same site only.** A stylesheet is read when its host is the home page's host or one of its subdomains, with a leading `www.` ignored on both. So `assets.example.com` is read for `example.com`, and `cdn.other.net` is not. We do not use the public suffix list: a site's own subdomains cover the common CDN case, and the rule is short enough to test completely.
- **The light screen view only.** A link marked `alternate`, or with a `media` list that is only for print, speech or a dark colour scheme, is skipped, because those colours are not what a visitor sees and the blog is light only.
- **Caps of our own** on top of `Outbound`'s: the home page at 2 MiB, the CSS from linked stylesheets at 4 MiB in all, and 16 linked stylesheets. Past a cap, a stylesheet is skipped, not the sync.
- **Only the home page can stop a sync.** A stylesheet that fails, is too large or is on another site is listed with its reason, and the rest are still read. The command line tells the user what was skipped.
- **`<style>` elements count**, in page order with the linked ones, because many sites put their colours there. `<base href>` is respected when links are resolved.
- **`@import` is not followed.** Google Fonts imports are recognised by the extraction pass from the text alone.
- **HTML is read with `scraper`**, as `docs/standards/rust.md` names for theme sync. It brings Servo's CSS parsers, which are MPL-2.0, so `deny.toml` names them as exceptions (BL-DEP-2).

## Consequences

- A site that serves its CSS from an unrelated CDN host gets the default colours for anything that lives only there. The user is told which links were skipped, and can paste their variables by hand.
- A sync reads at most about 6 MiB, so it finishes quickly even on a slow site and cannot be used to pull a large file through the server.
