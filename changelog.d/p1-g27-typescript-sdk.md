### Added

- A TypeScript SDK in `sdk/typescript/`, `@khalid/sdk`, generated from the API's OpenAPI document by the new `khalid-sdk` crate. `createClient` returns a client whose `request(operationId, input)` is typed for every Content API and Admin API operation, signs calls with the content key or the admin key, and answers the data or the server's problem. A test fails when the SDK no longer matches `openapi.json`, and another lists the demo posts through it from `khalid serve` (KH-API-4, task 1.20).
