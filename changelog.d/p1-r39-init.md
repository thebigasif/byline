### Fixed

- `khalid init` also makes `drafts/<language>/`, and its "Next" line starts a draft with `new --draft` and says how to publish it (KH-INS-2).
- The proxy rule `init` prints for nginx, Caddy, Apache or Traefik with no `--blog` now sends the blog to `http://127.0.0.1:8080`, where `khalid serve` answers, rather than a made-up host. For a hosted stack the rule still names a guessed `blog.` address, and `init` now says to change it, and `--json` lists it under `guessed` (KH-INS-3).
