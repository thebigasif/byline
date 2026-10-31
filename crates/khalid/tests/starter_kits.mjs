// Run by tests/starter_kits.rs: reads every post of a running Khalid site
// through a starter kit's khalid.js, the way the kit's pages do, and prints
// what the kit would write for each one as JSON.
//
// Usage: node starter_kits.mjs <path to khalid.js>, with KHALID_API_URL and
// KHALID_CONTENT_KEY set.

import { pathToFileURL } from "node:url";

const kit = await import(pathToFileURL(process.argv[2]).href);
const site = kit.siteFrom(process.env);
const shown = [];
for (const listed of await kit.getPosts(site)) {
  const post = await kit.getPost(site, listed.id);
  shown.push({
    url: listed.url,
    route: kit.routeOf(listed, "/blog/"),
    head: kit.headHtml(post.head),
    listed_head: kit.headHtml(listed.head),
    body: post.html,
  });
}
const missing = await kit.getPost(site, "en.no-such-post");
const settings = await kit.getSettings(site);
process.stdout.write(JSON.stringify({ posts: shown, missing, settings }));
