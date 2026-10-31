import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import type { MediaItem } from "../api/media";
import { en } from "../strings/en";

type Props = { media: MediaItem[] };

/** The kind of file in words, such as "Image". */
function kindName(kind: string): string {
  const kinds = en.media.kinds;
  return kind === "image" || kind === "video" || kind === "audio" ? kinds[kind] : kinds.file;
}

/** The kind, the pixels for an image, and the size, such as "Image, 1600 × 900, 48 KB". */
export function fileSummary(item: MediaItem): string {
  const parts = [kindName(item.kind)];
  if (typeof item.width === "number" && typeof item.height === "number") {
    parts.push(en.media.pixels(item.width, item.height));
  }
  parts.push(en.media.size(item.bytes));
  return parts.join(", ");
}

/** A small picture of an image, or its file type in letters for anything else. */
function preview(item: MediaItem): ReactNode {
  if (item.kind === "image") {
    // The name sits beside it, so the preview itself adds nothing for a screen reader.
    return <img className="thumb" src={item.url} alt="" width={96} height={64} loading="lazy" />;
  }
  const extension = item.name.split(".").pop() ?? "";
  return <span className="thumb none">{extension.toUpperCase()}</span>;
}

/** Whether an image has alt text, or that a file needs none. */
function altText(item: MediaItem): ReactNode {
  const words = en.media.library;
  if (item.kind !== "image") {
    return <span className="pill quiet">{words.notNeeded}</span>;
  }
  return item.alt === "" ? <span className="pill warn">{words.missing}</span> : item.alt;
}

/** One page of the library: a preview, the name, folder, kind and alt text of each file. */
export function MediaTable({ media }: Props) {
  const words = en.media.library;
  return (
    <div className="scroll">
      <table aria-labelledby="library-title">
        <thead>
          <tr>
            <th scope="col">{words.preview}</th>
            <th scope="col">{words.name}</th>
            <th scope="col">{words.folder}</th>
            <th scope="col">{words.kind}</th>
            <th scope="col">{words.alt}</th>
            <th scope="col">
              <span className="muted">{words.action}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {media.map((item) => (
            <tr key={item.id}>
              <td>{preview(item)}</td>
              <td data-label={words.name}>
                <code>{item.name}</code>
              </td>
              <td data-label={words.folder}>
                {item.folder === "" ? (
                  <span className="muted">{words.top}</span>
                ) : (
                  <code>{item.folder}</code>
                )}
              </td>
              <td data-label={words.kind}>{fileSummary(item)}</td>
              <td data-label={words.alt}>{altText(item)}</td>
              <td>
                <Link
                  to="/media/$id"
                  params={{ id: item.id }}
                  className="btn"
                  aria-label={words.edit(item.name)}
                >
                  {words.editText}
                </Link>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
