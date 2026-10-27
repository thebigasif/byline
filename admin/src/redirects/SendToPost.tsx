import { Link } from "@tanstack/react-router";
import { type FormEvent, type ReactNode, useState } from "react";
import { problemMessage } from "../api/posts";
import { isFailure, isPage, type PostRow, useRows } from "../api/rows";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { addAlias, isAlias } from "./alias";

/** The screen's one focus: an old address sent to a live post, kept in that post's file. */
export function SendToPost() {
  const words = en.redirects.send;
  // The list caps at 100, the most recently changed; titles are then sorted for the eye.
  const rows = useRows({ filter: "status:published", order: "updated_at desc", limit: 100 });
  const [from, setFrom] = useState("");
  const [id, setId] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<{ from: string; post: PostRow } | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);
  const posts = isPage(rows)
    ? [...rows.rows].sort((a, b) => a.title.localeCompare(b.title, en.locale))
    : [];

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const old = from.trim();
    const post = posts.find((each) => each.id === id);
    setDone(null);
    if (!isAlias(old)) {
      setFailed(en.problems.badAlias);
      return;
    }
    if (post === undefined || busy) {
      return;
    }
    setBusy(true);
    const answer = await addAlias(post.id, old);
    setBusy(false);
    if ("post" in answer) {
      setFailed(null);
      setFrom("");
      setDone({ from: old, post });
    } else {
      setFailed(problemMessage(answer.problem, en.problems.unreachable));
    }
  };

  let said: ReactNode = busy ? words.adding : null;
  if (done !== null) {
    said = (
      <>
        {words.done(done.from)}{" "}
        <Link to="/posts/$id" params={{ id: done.post.id }}>
          {done.post.title}
        </Link>
        . {words.where}
      </>
    );
  }
  let form: ReactNode = <p role="status">{words.loading}</p>;
  if (isFailure(rows)) {
    form = <Problem message={problemMessage(rows.problem, en.problems.unreachable)} />;
  } else if (isPage(rows) && posts.length === 0) {
    form = <p>{words.noPosts}</p>;
  } else if (isPage(rows)) {
    form = (
      <form onSubmit={submit}>
        <div className="pair">
          <div className="field">
            <label htmlFor="alias-from">{words.from}</label>
            <span className="hint" id="alias-from-hint">
              {words.fromHint}
            </span>
            <input
              id="alias-from"
              value={from}
              required
              aria-describedby="alias-from-hint"
              onChange={(event) => setFrom(event.target.value)}
            />
          </div>
          <div className="field">
            <label htmlFor="alias-to">{words.to}</label>
            <span className="hint" id="alias-to-hint">
              {words.toHint}
            </span>
            <select
              id="alias-to"
              value={id}
              required
              aria-describedby="alias-to-hint"
              onChange={(event) => setId(event.target.value)}
            >
              <option value="">{words.pick}</option>
              {posts.map((post) => (
                <option key={post.id} value={post.id}>
                  {post.title}
                </option>
              ))}
            </select>
          </div>
        </div>
        <p className="muted">{words.permanent}</p>
        <button type="submit" className="btn primary">
          {words.submit}
        </button>
      </form>
    );
  }
  return (
    <section className="focus wide" aria-labelledby="send-title">
      <h2 id="send-title">{words.title}</h2>
      <p>{words.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{said}</p>
      {form}
    </section>
  );
}
