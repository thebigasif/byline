### Changed

- `khalid serve` no longer renders and holds the body of every post for the Content API. The API keeps each post's details from the index, and a request renders the bodies of the posts it returns from their files, so start-up and saves do less work and the server holds less memory (KH-API-1, KH-OPS-4).
