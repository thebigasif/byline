// @ts-check
// Run by crates/khalid/tests/sdk_typescript.rs against `khalid serve` on the demo site.
import assert from "node:assert/strict";
import { test } from "node:test";
import { createClient } from "../src/index.js";

const baseUrl = process.env.KHALID_URL ?? "";
const contentKey = process.env.KHALID_CONTENT_KEY ?? "";

test("the SDK lists the demo posts in English, newest first", async () => {
  const khalid = createClient({ baseUrl, contentKey });
  const answer = await khalid.request("browse_posts", { query: { filter: "language:en" } });
  assert.ok(answer.ok, JSON.stringify(answer));
  assert.deepEqual(
    answer.data.posts.map((post) => post.slug),
    [
      "planning-a-topic-cluster",
      "fixing-slow-pages",
      "what-a-sitemap-does",
      "how-to-write-a-title-tag",
      "why-internal-links-matter",
    ],
  );
});

test("the SDK fills a path value and reads one post", async () => {
  const khalid = createClient({ baseUrl, contentKey });
  const answer = await khalid.request("read_post_by_slug", { path: { slug: "fixing-slow-pages" } });
  assert.ok(answer.ok, JSON.stringify(answer));
  assert.equal(answer.data.posts[0]?.slug, "fixing-slow-pages");
});

test("without a key the SDK hands back the server's problem", async () => {
  const khalid = createClient({ baseUrl });
  const answer = await khalid.request("browse_posts", {});
  assert.ok(!answer.ok, JSON.stringify(answer));
  assert.equal(answer.status, 401);
  assert.ok(answer.problem.fix.length > 0, JSON.stringify(answer.problem));
});
