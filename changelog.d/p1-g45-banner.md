### Added

- When the site's `[code]` waits for consent, every served page ends with a banner, a plain form with Allow and Do not allow that works with scripts off, in English and German. The site's code and a post's own head code are marked for the server to add only for a reader who said yes. With `consent = "not-needed"` the code goes into every page, and `khalid build` writes it without a nonce (G45).
