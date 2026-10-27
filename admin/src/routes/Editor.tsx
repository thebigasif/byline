import { useQuery } from "@tanstack/react-query";
import { getRouteApi } from "@tanstack/react-router";
import { readPost } from "../api/posts";
import { Problem } from "../components/Problem";
import { EditorForm } from "../editor/EditorForm";
import { en } from "../strings/en";

const route = getRouteApi("/signed-in/posts/$id");

/** Opens one post by its identity and shows it in the editor. */
export function Editor() {
  const { id } = route.useParams();
  const post = useQuery({ queryKey: ["post", id], queryFn: () => readPost(id) });

  if (post.isPending) {
    return <p role="status">{en.editor.loading}</p>;
  }
  const answer = post.data;
  if (answer === undefined || !("post" in answer)) {
    const message =
      answer !== undefined && answer.status === 404
        ? en.problems.notFound
        : en.problems.unreachable;
    return (
      <>
        <h1>{message.what}</h1>
        <Problem message={message} />
      </>
    );
  }
  return <EditorForm key={answer.post.id} post={answer.post} />;
}
