### Added

- The outbound client has a reach for the site under audit, public addresses and loopback and nothing else inward, so `khalid audit` can crawl `khalid serve` on the same machine, and a limit of no redirects, which hands each redirect back as the answer (KH-LNT-5, BL-RUN-8).
