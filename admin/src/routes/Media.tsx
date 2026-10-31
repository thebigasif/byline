import { getRouteApi } from "@tanstack/react-router";
import { Library } from "../media/Library";
import { en } from "../strings/en";

const route = getRouteApi("/signed-in/media");

/** The media library: every file already there. */
export function Media() {
  const search = route.useSearch();
  return (
    <>
      <h1>{en.media.title}</h1>
      <p className="lede">{en.media.lede}</p>
      <Library search={search} />
    </>
  );
}
