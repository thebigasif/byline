import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { PostRow } from "../api/rows";
import { en } from "../strings/en";
import { BulkChange } from "./BulkChange";
import type { BulkResult } from "./bulk";
import { PostTable } from "./PostTable";

type Props = { rows: PostRow[] };

/** The page of posts with a tick box on each, and one form that changes the ticked ones. */
export function BulkPosts({ rows }: Props) {
  const queries = useQueryClient();
  const [ticked, setTicked] = useState<string[]>([]);
  const [result, setResult] = useState<BulkResult | null>(null);
  const authors = [...new Set(rows.flatMap((row) => (row.author ? [row.author] : [])))];
  const chosen = rows.filter((row) => ticked.includes(row.id));

  const tick = (id: string, on: boolean) => {
    setTicked((now) => (on ? [...now, id] : now.filter((kept) => kept !== id)));
  };
  const done = async (came: BulkResult) => {
    setResult(came);
    setTicked([]);
    await queries.invalidateQueries({ queryKey: ["rows"] });
  };

  return (
    <>
      <div role="status">
        {result === null ? null : (
          <>
            <p>{en.bulk.done(result.changed)}</p>
            {result.left.map((why) => (
              <p key={why}>{why}</p>
            ))}
          </>
        )}
      </div>
      {chosen.length === 0 ? null : <BulkChange ticked={chosen} authors={authors} onDone={done} />}
      <PostTable rows={rows} ticked={ticked} onTick={tick} />
    </>
  );
}
