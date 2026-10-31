### Fixed

- `khalid serve` no longer fires a second `.published` webhook, or a false `.deleted`, for an entry saved while the server was still starting. The full render of the content read at start now hands nothing to the Content API, webhooks or `IndexNow` when a render of newer content already has.
