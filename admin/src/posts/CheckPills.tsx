import type { PostRow } from "../api/rows";
import { en } from "../strings/en";

type Props = Pick<PostRow, "lint">;

/** A post's lint findings as counted pills: blocks first, or one "no problems". */
export function CheckPills({ lint }: Props) {
  if (lint.blocks === 0 && lint.warnings === 0) {
    return <span className="pill ok">{en.posts.clear}</span>;
  }
  return (
    <span className="flex flex-wrap gap-1">
      {lint.blocks === 0 ? null : (
        <span className="pill block">{en.posts.blocks(lint.blocks)}</span>
      )}
      {lint.warnings === 0 ? null : (
        <span className="pill warn">{en.posts.warnings(lint.warnings)}</span>
      )}
    </span>
  );
}
