### Added

- `khalid lint` warns when a post's web address holds a letter outside a to z, such as `café-menus`, on a site that keeps plain addresses (`slugs = "ascii"` in `site.toml`, the default). The warning names the plain form, `cafe-menus`, because accented letters turn into codes such as `%C3%A9` when an address is copied. Sites with `slugs = "unicode"` are never warned (KH-LNT-2).
