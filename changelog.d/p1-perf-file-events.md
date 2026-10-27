Changed
`khalid serve` now hears a saved file from the system the moment it is written, instead of looking at every file twice a second, so an edit reaches the renderer in about 10 ms and an idle server does no work. Where the system gives no file events, such as on some network drives, it still looks every half second (BL-PERF-1, KH-OPS-4).
