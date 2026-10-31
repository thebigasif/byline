import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { listPosts, type Rows } from "./rows";

/** How many posts a link search or a mention offers at once. */
export const FOUND_LENGTH = 8;

/** Quotes `words` for the list grammar: a backslash before `'` and `\`. */
function quoted(words: string): string {
  return `'${words.replace(/[\\']/g, (found) => `\\${found}`)}'`;
}

/** The filter for posts whose title or slug holds `words`; none lists every post. */
export function searchFilter(words: string): string | undefined {
  const wanted = words.trim();
  return wanted === "" ? undefined : `title:~${quoted(wanted)},slug:~${quoted(wanted)}`;
}

/** Finds the posts a writer means by `words`, the latest change first. */
export function findPosts(words: string): Promise<Rows> {
  return listPosts({ filter: searchFilter(words), order: "updated_at desc", limit: FOUND_LENGTH });
}

/** `value` once it has stayed the same for `ms`, so a search waits for a pause. */
function useSettled(value: string, ms: number): string {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return settled;
}

/** The posts that match `words`, kept on screen while the next search runs. */
export function usePostSearch(words: string): Rows | undefined {
  const settled = useSettled(words, 150);
  return useQuery({
    queryKey: ["post-search", settled],
    queryFn: () => findPosts(settled),
    placeholderData: keepPreviousData,
  }).data;
}
