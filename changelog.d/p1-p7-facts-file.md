### Added

- A fact registry: a site can list its facts, such as a price, in `facts.toml` with the values they held before and the day each was last checked. `khalid lint` reads it and holds every post to it, and a broken file is refused with what, why and fix.
