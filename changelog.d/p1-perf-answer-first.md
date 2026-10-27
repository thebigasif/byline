### Changed

- `khalid serve` answers its home page before it has rendered every post, then renders the rest at once behind it. On a 1,000-post site the first page comes about three times sooner. A request for another page in that moment waits for it, and a site whose pages cannot be rendered still stops with the reason (KH-OPS-4).
