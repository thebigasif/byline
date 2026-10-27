import { Link } from "@tanstack/react-router";
import type { Pagination } from "../api/rows";
import { en } from "../strings/en";
import type { PostSearch } from "./filters";

type Props = { pagination: Pagination; search: PostSearch };

/** Links to the pages before and after this one, keeping the filters; nothing for one page. */
export function Paging({ pagination, search }: Props) {
  const { page, pages, prev, next } = pagination;
  if (pages <= 1) {
    return null;
  }
  return (
    <nav className="paging" aria-label={en.paging.label}>
      {prev === null || prev === undefined ? null : (
        <Link to="/posts" search={{ ...search, page: prev > 1 ? prev : undefined }} className="btn">
          {en.paging.previous}
        </Link>
      )}
      <span>{en.paging.where(page, pages)}</span>
      {next === null || next === undefined ? null : (
        <Link to="/posts" search={{ ...search, page: next }} className="btn">
          {en.paging.next}
        </Link>
      )}
    </nav>
  );
}
