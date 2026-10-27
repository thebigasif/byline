### Changed

- The linter now checks the structured data each post's page carries. It builds the post's schema graph the way `khalid build` does and passes every property Google needs and the graph lacks to the `invalid-structured-data` rule, which blocks publishing. Posts built today carry nothing that can lack such a property, so nothing new is refused; the rule fires as soon as block schema reaches pages (KH-LNT-1, KH-SEO-3).
