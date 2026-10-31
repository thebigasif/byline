/** One line of a comparison: in both texts, added since, or removed since. */
export type DiffLine = { kind: "same" | "added" | "removed"; text: string };

/**
 * The lines of `before` and `after` in order, each marked as kept, added
 * in `after` or removed from `before`, keeping the longest run of lines
 * the two share, so a change shows as few lines as it can.
 */
export function lineDiff(before: string, after: string): DiffLine[] {
  const a = before.split("\n");
  const b = after.split("\n");
  const width = b.length + 1;
  // shared[i * width + j]: how many lines `a` from i and `b` from j share in order.
  const shared = new Uint32Array((a.length + 1) * width);
  const at = (i: number, j: number) => shared[i * width + j] ?? 0;
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      shared[i * width + j] =
        a[i] === b[j] ? at(i + 1, j + 1) + 1 : Math.max(at(i + 1, j), at(i, j + 1));
    }
  }
  const lines: DiffLine[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    const left = a[i];
    const right = b[j];
    if (left !== undefined && left === right) {
      lines.push({ kind: "same", text: left });
      i += 1;
      j += 1;
    } else if (left !== undefined && (right === undefined || at(i + 1, j) >= at(i, j + 1))) {
      // A removed line comes before the line that replaced it.
      lines.push({ kind: "removed", text: left });
      i += 1;
    } else {
      lines.push({ kind: "added", text: right ?? "" });
      j += 1;
    }
  }
  return lines;
}
