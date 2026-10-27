### Changed

- `khalid serve` builds the Content API's index and lists from the content it already read, instead of reading every file again, and finds each post's file by its path instead of searching the whole list, so the work grows with the number of posts, not its square (KH-API-1, KH-OPS-4).
