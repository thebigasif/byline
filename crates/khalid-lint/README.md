# khalid-lint

The linter behind [Khalid](https://github.com/thebigasif/khalid), the blog engine by SerpDad. It reads a post and its site, and returns findings: blocks that stop a publish, and warnings that do not. Every finding says what is wrong, why it matters and how to fix it, in plain words.

The rules and their messages live in `catalogue.toml`, shipped inside the crate. A site can turn rules down or off in its `lint.toml`.

## Licence

AGPL-3.0-only, like the rest of Khalid. You may use, change and host it. If you change it and let others use the changed version over a network, you share your changes. See `docs/adr/` in the repository for why these crates are published under the same licence as the product.

Khalid and SerpDad are trademarks; the licence covers the code, not the name.
