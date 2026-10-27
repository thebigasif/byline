### Changed

- `khalid serve` answers its pages before the Content API is ready. The API's posts and the index in `data/site.db` are made in the background after each render, so neither the first page nor a saved post waits for them, and an API request made in the first moments waits for the posts instead of answering an empty list (KH-API-1, KH-OPS-4).
