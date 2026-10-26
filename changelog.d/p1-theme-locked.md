### Added

- `theme/tokens.toml` takes a `locked` list at the top, such as `locked = ["colors.link"]`. It names the values you set by hand, so the next theme sync keeps them. The blog checks each name and points to the line of a name it does not know.
