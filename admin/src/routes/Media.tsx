import { getRouteApi } from "@tanstack/react-router";
import { AddFile } from "../media/AddFile";
import { Library } from "../media/Library";
import { en } from "../strings/en";

const route = getRouteApi("/signed-in/media");

/** The media library: a file in, and every file already there. */
export function Media() {
  const search = route.useSearch();
  return (
    <>
      <h1>{en.media.title}</h1>
      <p className="lede">{en.media.lede}</p>
      <AddFile />
      <Library search={search} />
    </>
  );
}
