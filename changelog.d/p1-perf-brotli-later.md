Changed
`khalid serve` answers as soon as the pages are rendered, then adds their Brotli copies in the background. On the 1,000-post site the first page arrived about half a second sooner on the build server, and the server holds about half the memory once idle. Until the copies are ready, which takes well under a second, pages are sent uncompressed (BL-PERF-1, KH-OPS-4).

Fixed
A post saved while `khalid serve` was starting could stay unseen until the next save. The server now notes every file before its first render, so any later save shows (BL-RUN-10).
