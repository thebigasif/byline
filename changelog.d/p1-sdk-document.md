Added
`sdk/typescript/openapi.json` holds the OpenAPI document the API serves at `/api/openapi.json`, the source the TypeScript SDK is generated from. A test rewrites it and fails when the routes change, so it never drifts (KH-API-4).
