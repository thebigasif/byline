# Phase 5 answers, 2026-10-27

The founder said on 2026-10-27: "you got my all yes. dont stop any work for my yes." These answers settle the questions in #438, #439 and #441.

## Comments (#439)

1. Khalid ships its own comments with moderation. A Giscus option can come later. Plan D-4 is settled.
2. The task is 5.4 in `docs/phases/phase-5.md`.
3. The webhook keeps its catalogue name, `comment.added`.
4. A reader gives a name. An email is optional and is never shown.
5. Rate limits use a salted hash of the network address. The salt rotates daily and the hash is kept 24 hours. The raw address is never stored.
6. The comments crate owns the logic. It may add small Admin API routes in `khalid-api` and a `Comment` hook in `khalid-seo`, each in its own small pull request.
7. The reader form and the Admin API come first. The admin Comments screen comes from the mockups track.

## Newsletter (#441)

1. The tasks are 5.5 to 5.12 in `docs/phases/phase-5.md`.
2. A post is emailed only when its writer ticks "send as email". It is sent once, and an edit never sends it again.
3. Numbers are totals per email, never per person. Open counting is off by default, and click counting is a site setting.
4. Khalid refuses to send until the site has a postal address, and says why.
5. The newsletter track adds a forward migration for what the subscriber tables need.
6. Mail settings live in `site.toml`. Secrets come only from environment variables such as `KHALID_SMTP_PASSWORD`, never from files in the site folder.
7. The `::newsletter` block gains `list=`. Welcome emails live in `content/newsletters/<name>/welcome.md`.
8. SMTP uses `lettre` (MIT). A private or local SMTP host is allowed only when `site.toml` names it explicitly. Resend and Postmark go through `Outbound`.
9. The Subscribers screen comes from the mockups track.

## Deploy (#438)

1. The tasks are 5.13 and 5.14 in `docs/phases/phase-5.md`.
2. The deploy track may change `khalid_server::serve` to accept a TLS listener, in a small pull request.
3. Certificate tests run against `pebble` when it is present and are ignored otherwise. The deploy track proposes the CI job in its pull request body, and the CTO adds it.
4. The release builds gain fully static musl targets for Linux. The deploy track proposes the `release.yml` change and the CTO applies it.
5. The first `khalid serve --domain` asks the user to agree to Let's Encrypt's subscriber agreement and prints its link. `--accept-terms` answers it without a terminal. The answer is stored, so the question is asked once.
6. The idle memory budget stays defined on plain HTTP. HTTPS gets its own measure, observed and not gated.
