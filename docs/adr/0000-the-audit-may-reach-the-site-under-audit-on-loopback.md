# 0000. The audit may reach loopback for the site under audit, and nothing else inward

**Number:** next free. **Status:** Proposed, 2026-10-27. Audit 2, brief G30 (KH-LNT-5); RULES.md BL-RUN-8.

## Context

`khalid audit --url` crawls a running blog over HTTP, to check what only a server sends: status codes as the server gives them, redirects it answers itself, and headers such as `Strict-Transport-Security`. The blog a person audits this way is most often `khalid serve` on the same machine, at `http://127.0.0.1:8080/blog/`. The one outbound client (BL-RUN-8) refuses loopback, private and link-local addresses, so a pasted link can never reach into the network Khalid runs in. Brief G30 asks for loopback to be allowed only for the site under audit, with an ADR.

## Decision

- `khalid-outbound` gains `Reach::SiteUnderAudit` and `Resolver::for_site_under_audit()`: public addresses and loopback (`127.0.0.0/8` and `::1`). Private ranges, link-local addresses, the cloud metadata address and every other inward range stay refused.
- Only `khalid audit` builds a client with this reach, and only for the site it was pointed at. It asks that client for addresses on the blog's own origin, scheme, host and port, and nothing else: links that leave the blog are checked with the ordinary public client.
- That client follows no redirect (`Limits::with_redirects(0)`): each redirect comes back as the answer, and the crawl decides whether its target is still inside the blog before asking for it. So a redirect cannot carry the audit to another port or host on the machine.
- The guard checks addresses, not ports, so the origin check in the crawl is what keeps the audit on the blog's own port.

## Consequences

- A person can audit the blog they are running locally, which is the common case, without turning the guard off anywhere else.
- A blog on another machine of a private network, such as `192.168.1.5`, cannot be audited over HTTP; audit its built folder instead, or its public address.
- Every other client keeps `Reach::Public`, and `Reach::PublicAndLoopback` stays a test-only reach.
