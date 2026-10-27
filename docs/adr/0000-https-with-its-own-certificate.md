# 0000. `khalid serve --domain` gets and renews its own certificate

**Status:** Proposed, 2026-10-27. Phase 5 task 5.13. Number: next free.

## Context

Plan section 4.7 says `khalid serve --domain blog.example.com` gets its own HTTPS certificate through Let's Encrypt and renews it, with no reverse proxy for the simple case (KH-INS-4). The Phase 5 answers (`docs/decisions/2026-10-27-phase-5-answers.md`, deploy) settle four points: the server may take a TLS listener; certificate tests run against pebble when it is present; the first `--domain` asks the owner to agree to Let's Encrypt's subscriber agreement, `--accept-terms` answers it without a terminal, and the answer is stored; HTTPS memory is observed, not gated. Everything else below is a technical choice the plan leaves open.

## Decision

- **One crate, `khalid-https`,** owns HTTPS: the certificate read from PEM, the TLS listener, the answers to Let's Encrypt's checks, the ACME account and order, the saved files, renewal, and `Https::start`, which the CLI calls. The CLI adds one module, `serve_https.rs`, and its messages.
- **ACME through `instant-acme` over `Outbound`.** `instant-acme` takes our own HTTP client, so every request to Let's Encrypt goes through `Outbound::exchange` with its timeouts, size cap and private-network guard (BL-RUN-8). `rustls-acme` brings its own client and its own timer, so it could meet neither BL-RUN-8 nor BL-RUN-1. Everything uses `ring`, as the rest of the tree does; no `aws-lc-rs`.
- **Both checks.** TLS-ALPN-01 is tried first, since it needs port 443 only; HTTP-01 on port 80 is the fallback. Port 80 also moves every other request to HTTPS with a 308, which keeps a form's POST a POST.
- **The first certificate comes before the site answers.** With no good saved certificate, `start` orders one while the TLS listener accepts, so the TLS check can be answered; visitors in those seconds are turned away. A good saved certificate is served at once, and a Let's Encrypt outage never stops a start: the account signs in again every hour in the background.
- **Renewal at two thirds of a certificate's life,** Let's Encrypt's own advice, so a 90-day certificate renews with 30 days left and a shorter one keeps the same share. The loop looks at least every 12 hours, takes its time from `Clock` (BL-RUN-1), and waits an hour after a failure. A renewal is served from the next connection, without a restart.
- **Files in `data/https/`** in the site folder (BL-FS-2): `account.json`, `certificate.pem`, `key.pem`, each written whole under a new name with mode `0600`, then renamed into place. The saved account is the stored answer to the terms: an account exists only after the owner agreed.
- **The terms question.** With no saved account, `khalid serve --domain` asks at a terminal and shows the agreement's address; only `yes` or `y` is a yes. With no terminal, as in a service or a container, it stops and says to add `--accept-terms`. No contact email is sent to Let's Encrypt: it no longer sends expiry mail, and Khalid renews by itself.
- **Addresses.** HTTPS listens on port 443 and plain HTTP on port 80, on every IPv6 address with IPv4 alongside, falling back to every IPv4 address where IPv6 is off. Hidden flags `--https-listen`, `--http-listen` and `--acme-directory` exist for tests, for port forwarding, and for Let's Encrypt's staging directory.
- **Static mode** refuses `--domain` with the reason: the static host gives the files HTTPS.

## Consequences

- Ports below 1024 need permission on Linux. The message names the systemd unit (task 5.14) and `setcap cap_net_bind_service`.
- The release binary grows by the ACME and certificate code. Its size is measured in the pull request against the 30 MB budget.
- The pebble tests are ignored on machines without pebble. The CI job that runs them is proposed in #470 for the CTO.
- Deleting `data/https/` is always safe: Khalid opens a new account (after asking again) and gets a new certificate at the next start.
