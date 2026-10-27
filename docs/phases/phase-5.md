# Phase 5: server features and static mode

Pulled forward beside Phase 1 on 2026-10-27, with the founder's standing yes. Plan: `docs/plan.md` Phase 5 and section 4.7. Decisions: `docs/decisions/2026-10-27-phase-5-answers.md`. Every feature is a plain HTML form or page first, works with JavaScript blocked, and is switched off with a plain explanation in static mode.

## Tasks

### 5.1 Scheduled publish and preview links
- Status: `doing`.
- Publishing has one door, the Admin API publish operation; the scheduler calls it. Preview links are signed, expire, and carry `noindex`.
- Done when: a post scheduled for a fake-clock time publishes once, survives a restart, fires the webhook and IndexNow, and a preview link expires on time.

### 5.2 Search
- Status: `doing`.
- Done when: `/search/` answers a plain GET form with ranked, highlighted results in English and German, and static mode explains that search needs the server.

### 5.3 Static mode
- Status: `doing`.
- Done when: a static build of the demo site has no form that posts to a missing endpoint, and `khalid doctor` lists the features that are off.

### 5.4 Comments
- Status: `todo`.
- Our own comments with moderation (plan D-4 settled). Stored as visitor data, a plain form with a honeypot and rate limits, held for approval by default, threaded one level, Admin API routes to approve, reply and delete, the `comment.added` webhook, and `Comment` entries in the page JSON-LD.
- Done when: tests cover spam rejection, moderation, rate limits and the no-script form.

### 5.5 Subscribers and double opt-in
- Status: `todo`.
- A forward migration adds what the subscriber tables need: token hashes, lists per subscriber, and timestamps. Sign-up, the confirmation email, one-click unsubscribe.

### 5.6 Mail sending
- Status: `todo`.
- SMTP through `lettre`, plus Resend and Postmark through `Outbound`. Mail settings in `site.toml`; secrets only from the environment.

### 5.7 Newsletters and segments
- Status: `todo`.
- Several newsletters per site; the `::newsletter{list=...}` block names one.

### 5.8 Sending a post
- Status: `todo`.
- A post is emailed only when its writer asks, once, never again on an edit. Khalid refuses to send until the site has a postal address.

### 5.9 Welcome emails
- Status: `todo`.
- Each newsletter's welcome email is a markdown file in `content/newsletters/<name>/welcome.md`.

### 5.10 The embeddable sign-up form
- Status: `todo`.

### 5.11 Gated content and password pages
- Status: `todo`.

### 5.12 Newsletter numbers
- Status: `todo`.
- Totals per email only, never per person. Open counting is off by default.

### 5.13 HTTPS with its own certificate
- Status: `todo`.
- `khalid serve --domain` gets and renews a Let's Encrypt certificate. The server accepts a TLS listener.

### 5.14 Packages
- Status: `todo`.
- A systemd unit, a Docker image built from scratch on a fully static (musl) binary, a Compose file, and the install script.
