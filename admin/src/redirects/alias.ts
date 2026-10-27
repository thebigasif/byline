import { type Answer, readPost, savePost } from "../api/posts";

/** Whether `path` can be an old address in front matter: it starts with / and has no spaces. */
export function isAlias(path: string): boolean {
  return path.startsWith("/") && !/\s/.test(path);
}

/**
 * Adds `alias` to the old addresses of post `id`, on the copy just read, so
 * a change someone made since is never undone. An alias it already has is
 * left as it is.
 */
export async function addAlias(id: string, alias: string): Promise<Answer> {
  const read = await readPost(id);
  if (!("post" in read) || read.post.settings.aliases.includes(alias)) {
    return read;
  }
  const aliases = [...read.post.settings.aliases, alias];
  return savePost(id, { updated_at: read.post.updated_at, settings: { aliases } });
}
