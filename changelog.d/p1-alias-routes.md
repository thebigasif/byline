Fixed
A post alias now moves to the address `routes.toml` gives the post, in the `_redirects` file `khalid build` writes and in the 301 `khalid serve` answers. Before, a collection routed as `/{year}/{slug}/` had every alias point at `/{slug}/`, a page that does not exist (KH-SEO-8, audit G2).
