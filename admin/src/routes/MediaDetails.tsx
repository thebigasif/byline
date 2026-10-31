import { useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { useState } from "react";
import { readMedia } from "../api/media";
import { isData } from "../api/redirects";
import { Problem } from "../components/Problem";
import { DetailsForm } from "../media/DetailsForm";
import { fileSummary } from "../media/MediaTable";
import { day } from "../strings/dates";
import { en } from "../strings/en";

const route = getRouteApi("/signed-in/media/$id");

/** One file in the library: what it is, its address to copy, and its details to change. */
export function MediaDetails() {
  const words = en.media.details;
  const { id } = route.useParams();
  const reply = useQuery({ queryKey: ["media-item", id], queryFn: () => readMedia(id) });
  const [copied, setCopied] = useState(false);

  const back = (
    <p>
      <Link to="/media" search={{}}>
        {words.back}
      </Link>
    </p>
  );
  if (reply.isPending) {
    return <p role="status">{words.loading}</p>;
  }
  if (!isData(reply.data)) {
    const message = reply.data?.status === 404 ? en.problems.noMediaItem : en.problems.unreachable;
    return (
      <>
        {back}
        <h1>{message.what}</h1>
        <Problem message={message} />
      </>
    );
  }
  const item = reply.data.data;
  // Browsers allow copying only over HTTPS or on localhost; the address stays on screen anyway.
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(item.url);
      setCopied(true);
    } catch {
      setCopied(false);
    }
  };
  return (
    <>
      {back}
      <h1>{item.name}</h1>
      <p className="lede">
        {words.uploaded(fileSummary(item), item.uploaded_by, day(item.uploaded_at))}
      </p>
      <p>
        {words.address} <code>{item.url}</code>{" "}
        <button type="button" className="btn" onClick={copy}>
          {words.copy}
        </button>
      </p>
      <p role="status">{copied ? words.copied : null}</p>
      <DetailsForm key={item.id} item={item} />
    </>
  );
}
