# Khalid TypeScript SDK

A TypeScript client for the Khalid Content API and Admin API, generated from the API's own OpenAPI document (task 1.20, KH-API-4). No API type in it is written by hand.

## Where the document comes from

`openapi.json` here is the document a Khalid server answers at `/api/openapi.json`. The test `crates/khalid-api/tests/sdk_document.rs` keeps it current: when the routes change, the test rewrites the file and fails once, so you see the change and commit it.

To refresh it by hand:

```sh
cargo nextest run -p khalid-api --test sdk_document
```

Run it a second time to see it pass.
