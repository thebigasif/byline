/**
 * A post's name from its file in the site folder, as a person would say it:
 * `content/en/posts/2026-09-14-why-links-matter.md` gives `why-links-matter`.
 */
export function postName(file: string): string {
  const parts = file.split("/");
  let last = parts.at(-1) ?? file;
  if (last === "index.md" && parts.length > 1) {
    last = parts.at(-2) ?? last;
  }
  return last.replace(/\.md$/, "").replace(/^\d{4}-\d{2}-\d{2}-/, "");
}
