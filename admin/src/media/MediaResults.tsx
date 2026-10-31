import { useQuery } from "@tanstack/react-query";
import { listMedia } from "../api/media";
import { problemMessage } from "../api/posts";
import { isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { en } from "../strings/en";
import { MediaPaging } from "./MediaPaging";
import { MediaTable } from "./MediaTable";
import { type MediaSearch, mediaQuery } from "./search";

type Props = { search: MediaSearch };

/** What the list found: how many files and the page of them, or the problem. */
export function MediaResults({ search }: Props) {
  const words = en.media.library;
  const query = mediaQuery(search);
  const reply = useQuery({ queryKey: ["media", query], queryFn: () => listMedia(query) }).data;
  if (reply === undefined) {
    return <p role="status">{words.loading}</p>;
  }
  if (!isData(reply)) {
    return <Problem message={problemMessage(reply.problem, en.problems.unreachable)} />;
  }
  const total = reply.data.pagination.total;
  let found = words.found(total);
  if (total === 0) {
    found = search.show === "no-alt" ? words.allHaveAlt : words.empty;
  }
  return (
    <>
      <p role="status">{found}</p>
      {total === 0 ? null : <MediaTable media={reply.data.media} />}
      <MediaPaging page={reply.data} search={search} />
    </>
  );
}
