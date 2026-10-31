### Changed

- A downloaded font's fallback tuning is measured through the read-fonts parser, whose typed tables refuse a malformed file instead of reading it at raw offsets, and a file that is not a readable face is saved and served but never tuned, with its metrics left out rather than guessed.
