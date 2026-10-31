### Fixed

- `khalid serve` now fires `comment.added` to the webhooks that asked for it each time a reader leaves a comment, without the reader's email (KH-API-3, audit finding F5).

### Changed

- A webhook list that cannot be read no longer stops `khalid serve` from starting. It is logged when an event is sent, and each retry round reads it again.
