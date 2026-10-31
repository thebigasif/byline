import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { checkIn, leave, type PostPresence } from "../api/presence";

/** How often the editor says it still has the post open; the blog lets go after a minute. */
export const CHECK_IN_MS = 20_000;

/** Who has post `id` open, kept fresh while the editor shows it, and a way to take it over. */
export function usePresence(id: string) {
  const client = useQueryClient();
  const key = ["presence", id];
  const seen = useQuery({
    queryKey: key,
    queryFn: () => checkIn(id, "join"),
    refetchInterval: CHECK_IN_MS,
    refetchIntervalInBackground: true,
    staleTime: 0,
    retry: false,
  }).data;
  useEffect(() => {
    const gone = () => void leave(id);
    globalThis.addEventListener("pagehide", gone);
    return () => {
      globalThis.removeEventListener("pagehide", gone);
      gone();
    };
  }, [id]);
  const takeOver = async () => {
    try {
      const taken = await checkIn(id, "take_over");
      client.setQueryData<PostPresence | null>(key, taken);
    } catch {
      // The blog cannot be reached; the next check-in shows who holds the post.
    }
  };
  // Until the blog answers, or when it cannot say, nothing is locked.
  const presence: PostPresence | null = seen ?? null;
  return { presence, locked: presence !== null && !presence.yours, takeOver };
}
