### Added

- A post's head code goes into its page head after the SEO tags. On `khalid serve`, its scripts and styles get a nonce made fresh for each request, and that page's security policy carries the same nonce and lets scripts report to `https` addresses, so an analytics snippet such as GA4 runs on that post alone. The page is never cached. A static build writes the head code without a nonce (KH-EDT-3).
