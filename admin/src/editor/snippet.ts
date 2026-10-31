/** A post's address as a search result shows it: the site, then each folder after a ›. */
export function shownAddress(url: string): string {
  let address: URL;
  try {
    address = new URL(url);
  } catch {
    return url;
  }
  const folders = address.pathname.split("/").filter((folder) => folder !== "");
  return [address.origin, ...folders].join(" › ");
}

/** Each kind of thing a page's JSON-LD describes, once, in the order the graph lists them. */
export function schemaTypes(schema: Record<string, unknown>): string[] {
  const graph = schema["@graph"];
  const nodes: unknown[] = Array.isArray(graph) ? graph : [];
  const types = nodes.flatMap((node) => {
    const kind = typeof node === "object" && node !== null ? Reflect.get(node, "@type") : null;
    return typeof kind === "string" ? [kind] : [];
  });
  return [...new Set(types)];
}

/** Google's Rich Results Test for the page at `url`. */
export function richResultsTest(url: string): string {
  return `https://search.google.com/test/rich-results?url=${encodeURIComponent(url)}`;
}
