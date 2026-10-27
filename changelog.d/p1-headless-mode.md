Added
`site.toml` accepts `mode = "headless"` for a site whose pages another app renders from the Content API (KH-INS-5, ADR "headless is a mode"). A wrong `mode` now says to pick one of `server`, `static`, `headless`.
