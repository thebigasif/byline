# Khalid TypeScript SDK

A TypeScript client for the Khalid Content API and Admin API, generated from the API's own OpenAPI document (task 1.20, KH-API-4). No API type in it is written by hand. It has no runtime dependencies and runs anywhere `fetch` does: Node 18 or later, Deno, Bun and browsers.

## Use it

```js
import { createClient } from "@khalid/sdk";

const khalid = createClient({ baseUrl: "https://example.com", contentKey: "your content key" });

const answer = await khalid.request("browse_posts", { query: { filter: "language:en" } });
if (answer.ok) {
  for (const post of answer.data.posts) console.log(post.title);
} else {
  console.log(answer.problem.detail, answer.problem.fix);
}
```

Each call names an operation by its `operationId` in `openapi.json` and passes the values it takes: `path`, `query`, `headers` and `body`, or `{}` when it takes none. TypeScript knows what each operation takes and answers, so a wrong name or a missing value fails to compile. The answer is either `{ ok: true, data }` or `{ ok: false, problem }`, where the problem says what happened, why, and what to do (RFC 9457).

Content API calls send `contentKey` as `Authorization: Khalid-Content <key>`. Admin API calls send `adminKey`, an integration key or a staff token, as `Authorization: Khalid <key>`. Keep keys out of code that ships to a browser.

## What is generated

| File | What it holds | Written by |
|---|---|---|
| `openapi.json` | The document a Khalid server answers at `/api/openapi.json` | `crates/khalid-auth/tests/sdk_document.rs` |
| `src/schema.d.ts` | One type per schema, and what each operation takes and answers | `crates/khalid-sdk` |
| `src/operations.js` | Where each operation is sent and which key signs it | `crates/khalid-sdk` |

`src/client.js`, `src/index.js` and `src/index.d.ts` are written by hand and hold no API type.

When a route changes, the first test rewrites `openapi.json` and fails once, and the second rewrites the generated files and fails once. So a local run fixes both, and CI, which starts from the committed files, goes red until they are committed. To refresh them by hand, run each command until it passes:

```sh
cargo nextest run -p khalid-auth --test sdk_document
cargo nextest run -p khalid-sdk
```

## Checks

`crates/khalid/tests/sdk_typescript.rs` starts `khalid serve` on the demo site and runs `test/demo-posts.test.js` with Node against it, so the SDK is proven to list the demo posts. Node 18 or later must be on the `PATH` for that test.

Biome and the TypeScript compiler in strict mode check the hand-written files and the generated types:

```sh
npm install
npm run check
```
