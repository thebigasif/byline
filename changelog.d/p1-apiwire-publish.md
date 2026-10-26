### Added

- The Admin API publishes a draft when an edit sends `"status": "published"`. The linter checks the post first: a blocking finding keeps it a draft, and the answer is a 422 that lists every finding under `lint`. New posts take an `author`, since the linter refuses a post without one (KH-API-2, KH-LNT-3).
