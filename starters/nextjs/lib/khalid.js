// @ts-check
// Reads a Khalid site's Content API and turns each post's head into the
// tags the Khalid theme writes. No dependencies: it runs in Astro, Next.js
// and plain Node 18 or later. The Astro and Next.js starter kits carry the
// same copy of this file, and Khalid's tests check the two stay equal.

/**
 * @typedef {{ tag: "title", text: string }
 *   | { tag: "meta", name?: string, property?: string, content: string }
 *   | { tag: "link", rel: string, hreflang?: string, href: string }} HeadTag
 * @typedef {{ tags: HeadTag[], json_ld: string }} Head
 * @typedef {{
 *   id: string, slug: string, title: string, url: string, language: string,
 *   excerpt: string | null, featured: boolean, tags: string[],
 *   authors: string[], published_at: string, updated_at: string,
 *   head?: Head, html?: string
 * }} Post
 * @typedef {{
 *   title: string, url: string, default_language: string, languages: string[]
 * }} Settings
 * @typedef {{ api: string, key: string }} Site
 * @typedef {{ type: string, props: Record<string, unknown> }} Element
 */

/**
 * The Khalid site named by the environment: `KHALID_API_URL`, where
 * `khalid serve` answers, such as `http://127.0.0.1:2368`, and
 * `KHALID_CONTENT_KEY`, a key from `khalid keys create --kind content`.
 *
 * @param {Record<string, string | undefined>} env
 * @returns {Site}
 */
export function siteFrom(env) {
  const api = env.KHALID_API_URL;
  const key = env.KHALID_CONTENT_KEY;
  if (!api || !key) {
    throw new Error(
      "The starter kit does not know which Khalid site to read. " +
        "KHALID_API_URL or KHALID_CONTENT_KEY is not set. " +
        "Set both, as README.md shows, then build again.",
    );
  }
  return { api: api.replace(/\/+$/, ""), key };
}

/**
 * The site's public settings: its name, address and languages.
 *
 * @param {Site} site
 * @returns {Promise<Settings>}
 */
export async function getSettings(site) {
  const found = await read(site, "settings/", {});
  return found.settings[0];
}

/**
 * Every published post, newest first, in every language.
 *
 * @param {Site} site
 * @returns {Promise<Post[]>}
 */
export async function getPosts(site) {
  const found = await read(site, "posts/", { limit: "all" });
  return found.posts;
}

/**
 * The post with the identity `id`, with its HTML body, or `null` when
 * there is none.
 *
 * @param {Site} site
 * @param {string} id
 * @returns {Promise<Post | null>}
 */
export async function getPost(site, id) {
  const found = await read(site, `posts/${encodeURIComponent(id)}/`, {}, true);
  return found ? found.posts[0] : null;
}

/**
 * The path of `post` inside the blog's folder `base`, such as
 * `de/was-eine-sitemap-tut` for `/blog/de/was-eine-sitemap-tut/`, which a
 * catch-all route uses as its parameter.
 *
 * @param {Post} post
 * @param {string} base
 * @returns {string}
 */
export function routeOf(post, base) {
  const path = new URL(post.url).pathname;
  const folder = `/${base.replace(/^\/+|\/+$/g, "")}/`;
  const inside = path.startsWith(folder) ? path.slice(folder.length) : path;
  return inside.replace(/^\/+|\/+$/g, "");
}

/**
 * The elements of `head`, in order: each tag, then the JSON-LD script.
 * Props use React's names, so `hreflang` is `hrefLang`.
 *
 * @param {Head} head
 * @returns {Element[]}
 */
export function headElements(head) {
  /** @type {Element[]} */
  const elements = head.tags.flatMap((tag) => {
    switch (tag.tag) {
      case "title":
        return [{ type: "title", props: { children: tag.text } }];
      case "meta":
        return tag.name === undefined
          ? [{ type: "meta", props: { property: tag.property, content: tag.content } }]
          : [{ type: "meta", props: { name: tag.name, content: tag.content } }];
      case "link":
        return tag.hreflang === undefined
          ? [{ type: "link", props: { rel: tag.rel, href: tag.href } }]
          : [{ type: "link", props: { rel: tag.rel, hrefLang: tag.hreflang, href: tag.href } }];
      default:
        // A kind of tag newer than this file: left out, not guessed at.
        return [];
    }
  });
  elements.push({
    type: "script",
    props: { type: "application/ld+json", dangerouslySetInnerHTML: { __html: head.json_ld } },
  });
  return elements;
}

/**
 * `head` as HTML, one tag per line, exactly as the Khalid theme writes it.
 *
 * @param {Head} head
 * @returns {string}
 */
export function headHtml(head) {
  return headElements(head)
    .map(({ type, props }) => {
      if (type === "script") {
        const inner = /** @type {{ __html: string }} */ (props.dangerouslySetInnerHTML);
        return `<script type="application/ld+json">${inner.__html}</script>\n`;
      }
      if (type === "title") {
        return `<title>${escape(String(props.children))}</title>\n`;
      }
      const attributes = Object.entries(props)
        .map(([name, value]) => ` ${name.toLowerCase()}="${escape(String(value))}"`)
        .join("");
      return `<${type}${attributes}>\n`;
    })
    .join("");
}

/**
 * `text` with the five characters Khalid escapes in HTML written as
 * entities.
 *
 * @param {string} text
 * @returns {string}
 */
function escape(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

/**
 * The JSON the Content API answers at `path` with `query`. With
 * `missingIsNull`, a 404 gives `null`.
 *
 * @param {Site} site
 * @param {string} path
 * @param {Record<string, string>} query
 * @param {boolean} [missingIsNull]
 * @returns {Promise<any>}
 */
async function read(site, path, query, missingIsNull = false) {
  const url = new URL(`${site.api}/api/content/v1/${path}`);
  url.searchParams.set("key", site.key);
  for (const [name, value] of Object.entries(query)) {
    url.searchParams.set(name, value);
  }
  const answer = await fetch(url, { headers: { accept: "application/json" } });
  if (missingIsNull && answer.status === 404) {
    return null;
  }
  if (!answer.ok) {
    const shown = `${site.api}/api/content/v1/${path}`;
    throw new Error(
      `Khalid answered ${answer.status} for ${shown}. ` +
        "The starter kit cannot show posts it cannot read. " +
        "Check that khalid serve is running and the content key is right.",
    );
  }
  return answer.json();
}
