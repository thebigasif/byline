import { Link } from "@tanstack/react-router";
import type { MediaPage } from "../api/media";
import { en } from "../strings/en";
import type { MediaSearch } from "./search";

type Props = { page: MediaPage; search: MediaSearch };

/** Links to the pages before and after this one, keeping the switch; nothing for one page. */
export function MediaPaging({ page, search }: Props) {
  const { page: at, pages, prev, next } = page.pagination;
  if (pages <= 1) {
    return null;
  }
  return (
    <nav className="paging" aria-label={en.paging.label}>
      {prev === null || prev === undefined ? null : (
        <Link to="/media" search={{ ...search, page: prev > 1 ? prev : undefined }} className="btn">
          {en.paging.previous}
        </Link>
      )}
      <span>{en.paging.where(at, pages)}</span>
      {next === null || next === undefined ? null : (
        <Link to="/media" search={{ ...search, page: next }} className="btn">
          {en.paging.next}
        </Link>
      )}
    </nav>
  );
}
