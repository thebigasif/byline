import { useEffect, useState } from "react";
import { makePreviewLink, problemMessage } from "../api/posts";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

type Props = {
  id: string;
  /** When the post last saved; the page loads again each time it changes. */
  version: string;
};

/** The page's address on this blog: its path and query, without the site address. */
export function onThisBlog(url: string): string {
  const address = new URL(url);
  return `${address.pathname}${address.search}`;
}

/**
 * The post as readers will see it, in the site's own design, beside the
 * text. It shows the saved draft through a preview link, so it is the real
 * page and not an imitation, and loads again after each save.
 */
export function LivePage({ id, version }: Props) {
  const [address, setAddress] = useState<string | null>(null);
  const [problem, setProblem] = useState<UserMessage | null>(null);
  useEffect(() => {
    let current = true;
    void makePreviewLink(id)
      .catch(() => null)
      .then((answer) => {
        if (!current) {
          return;
        }
        if (answer !== null && "link" in answer) {
          setAddress(onThisBlog(answer.link.url));
        } else {
          setProblem(problemMessage(answer?.problem ?? null, en.problems.unreachable));
        }
      });
    return () => {
      current = false;
    };
  }, [id]);
  return (
    <section aria-labelledby="live-title">
      <h2 id="live-title">{en.live.title}</h2>
      <p className="hint">{en.live.hint}</p>
      {problem === null ? null : <Problem message={problem} />}
      {address === null ? null : (
        <iframe key={version} className="live-page" title={en.live.frame} src={address} />
      )}
    </section>
  );
}
