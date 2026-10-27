### Added

- A search reads the index in `data/site.db` for one language, ranks the posts that hold every word, and returns how many matched with the best of them, each with its address, title and text. A search looks up only its own words, so it costs about the same on a small blog or a large one (KH-WEB-2).
