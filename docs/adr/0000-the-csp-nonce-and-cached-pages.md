# The CSP nonce lives in the header, and cached pages stay the same bytes

**Number:** next free.

**Status:** Accepted, 2026-10-26. Phase 1 task 1.15.

## Context

BL-SEC-2 asks for a Content Security Policy on every response, with a nonce made fresh for each request and no `unsafe-inline` for scripts. The server sends pages rendered at publish, with an ETag from their bytes and a Brotli copy made once (ADR "the server answers from rendered pages in memory"). A nonce written into the HTML would change the bytes of every response. That would end ETags, 304s and the precompressed copy, and put rendering back on the request path.

## Decision

- **The nonce goes in the header only.** Every response, including redirects, 304s, 404s and 405s, carries `script-src 'nonce-<128 random bits>' 'strict-dynamic'`. The bits come from the operating system for each request. The cached page bytes never change.
- **The default templates have no inline script or style,** and the public site needs no JavaScript to read (BL-WEB-2). So the policy allows no script at all on today's pages. That is the strictest policy there is, and it stays true however the page is cached.
- **If random bytes are not available,** the script sources become `'none'`. A failure makes the policy stricter, never looser.
- **The rest of the policy:** `default-src 'none'`; styles, fonts, media and connections from the site; images from the site, any `https` address or `data:`, because writers and bookmark cards show pictures hosted elsewhere and an image cannot run code; `object-src 'none'`, `base-uri 'none'`, `form-action 'self'`, `frame-ancestors 'self'`. `frame-src` lists exactly the players `khalid-content` allows in `EMBED_PROVIDERS`. `khalid serve` passes them in, so the server does not depend on the content crate. Each must be `https://` and a host, so a bad entry cannot widen the policy.
- **The other headers:** `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`, `Referrer-Policy: strict-origin-when-cross-origin`, a `Permissions-Policy` that turns off camera, microphone, location, payment, USB and topics, and `Cross-Origin-Opener-Policy: same-origin`. `Strict-Transport-Security` waits for built-in HTTPS (KH-INS-4, Phase 5), because it means nothing over plain HTTP.

## Consequences

- A template that needs an inline script later must not print a nonce of its own. The page would mark the spot instead, and the server would fill it per request for that page only, without a tag or a precompressed copy. That is a new ADR when the first such script arrives.
- Each request costs about 1 µs more: one call for random bytes and one small string. The cached page still answers in about 2 µs in process.
- An admin that runs scripts (Phase 4) is served by its own routes with its own policy.
