# 0000. Mail leaves through khalid-mail

**Number:** next free. **Status:** Proposed, 2026-10-31. Phase 5 task 5.6, brief G38 in `docs/audits/2026-10-27-gaps-2.md`, KH-GRO-2.

## Context

Task 5.5 built sign-ups with double opt-in and a `Mailer` trait in `khalid-newsletter`, but no transport, so `khalid serve` answered every sign-up with "Signing up is not open yet". Sign-in links (G48) and gated content (G41) also need to send mail. The founder's answers of 2026-10-27 (decisions 6 and 8) settle the shape: settings in `site.toml`, secrets only in the environment, SMTP through `lettre`, Resend and Postmark through `Outbound`, and a private or local SMTP host only when `site.toml` names it.

## Decision

- **Settings in `khalid-core`.** `[mail]` holds `from`, an optional `name`, and exactly one of `[mail.smtp]`, `[mail.resend]` or `[mail.postmark]`. `Config` checks it at start. The secret comes from `KHALID_SMTP_PASSWORD`, `KHALID_RESEND_API_KEY` or `KHALID_POSTMARK_TOKEN` through `Environment::mail_secret`, and a missing one stops the start with the variable's name.
- **Transports in a new crate, `khalid-mail`.** `ApiMailer` (Resend, Postmark) posts through the shared `Outbound` client. `SmtpMailer` sends with `lettre` on rustls and ring. Each implements `khalid_newsletter::Mailer`, so every feature that sends mail calls one trait. `khalid-newsletter` keeps its rule that it does not know how mail leaves.
- **`khalid-mail` depends on `khalid-newsletter` for the trait**, rather than the trait moving down into `khalid-mail`. Moving `Mail`, `Mailer` and `MailError` would change the landed 5.5 code for no reader-visible gain. If a later feature needs mail without the newsletter crate, the trait moves then.
- **`private_host = true` is how `site.toml` names a private SMTP host.** Without it the host must resolve to public addresses through `khalid_outbound::Resolver`, and mail must be encrypted. With it, the name is looked up plainly and `security = "none"` is allowed. The connection goes to the checked address while TLS checks the certificate against the host's name, so a name cannot be pointed inward after the check.
- **STARTTLS on port 587 is the default and never falls back to plain text.**
- **No retries and no pool in the transport.** Sign-ups send one email at a time, and a failed confirmation can be asked for again after ten minutes. A bulk send (5.8) brings its own queue.
- **Nothing logs an email's text or a secret.** `SendError` holds the status or the cause, never the text, which carries working links. Keys stay in `Secret`.
- **`khalid serve` builds the mailer once at start** in `crates/khalid/src/newsletter.rs` and passes the newsletter its `SubscriberTable`.

## Consequences

- A site with `[mail]` takes sign-ups: the form stores the address, the confirmation leaves, and its link confirms. `crates/khalid/tests/serve_newsletter.rs` proves it end to end against `khalid_mail::StubSmtp`, a stand-in SMTP server behind `test-util`.
- Sign-in links (G48) need a small adapter from `khalid_auth::LinkSender` to `Mailer` in the binary; the transport is ready for it.
- `lettre` joins the binary. Resend and Postmark were checked against stand-in servers only, and SMTP over TLS against a real server was not run here, because this machine has no mail server to test with.
