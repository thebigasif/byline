import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { type FormEvent, useState } from "react";
import { createPost, problemMessage, requestKey } from "../api/posts";
import { en, type UserMessage } from "../strings/en";
import { Problem } from "./Problem";

/** Starts a draft from a working title and opens it in the editor. */
export function StartPost() {
  const navigate = useNavigate();
  const [title, setTitle] = useState("");
  const [problem, setProblem] = useState<UserMessage | null>(null);
  // One key per attempt, so a double click or a lost answer makes one draft.
  const [key, setKey] = useState(requestKey);
  const start = useMutation({ mutationFn: () => createPost(title.trim(), key) });

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const answer = await start.mutateAsync().catch(() => null);
    if (answer === null) {
      setProblem(en.problems.unreachable);
    } else if ("post" in answer) {
      await navigate({ to: "/posts/$id", params: { id: answer.post.id } });
      return;
    } else {
      setProblem(problemMessage(answer.problem, en.problems.unreachable));
    }
    setKey(requestKey());
  };

  return (
    <>
      {problem === null ? null : <Problem message={problem} />}
      <form className="focus" onSubmit={submit}>
        <h2 className="mt-0">{en.start.title}</h2>
        <p>{en.start.lede}</p>
        <div className="field">
          <label htmlFor="new-title">{en.start.label}</label>
          <input
            id="new-title"
            required
            value={title}
            onChange={(event) => setTitle(event.target.value)}
          />
        </div>
        <button type="submit" className="btn primary" disabled={start.isPending}>
          {start.isPending ? en.start.starting : en.start.submit}
        </button>
      </form>
    </>
  );
}
