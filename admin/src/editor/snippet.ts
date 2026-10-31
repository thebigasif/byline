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
