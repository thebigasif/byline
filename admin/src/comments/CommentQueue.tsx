import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { isData, type Reply } from "../api/call";
import {
  type AdminComment,
  approveComment,
  type CommentStatus,
  deleteComment,
  listComments,
  replyToComment,
} from "../api/comments";
import { problemMessage } from "../api/posts";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { CommentCard } from "./CommentCard";

type Props = { status: CommentStatus };

/** The comments with one status, newest first, each with what can be done to it. */
export function CommentQueue({ status }: Props) {
  const words = en.comments;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: ["comments", status], queryFn: () => listComments(status) });
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  // Says how a call went and reads the list again; true when it worked.
  const settle = async <T,>(answer: Reply<T>, said: string): Promise<boolean> => {
    if (!isData(answer)) {
      setDone(null);
      setFailed(problemMessage(answer.problem, en.problems.unreachable));
      return false;
    }
    setFailed(null);
    setDone(said);
    await queries.invalidateQueries({ queryKey: ["comments"] });
    return true;
  };
  const approve = async (comment: AdminComment) => {
    await settle(await approveComment(comment.id), words.approved(comment.author_name));
  };
  const remove = async (comment: AdminComment) => {
    await settle(await deleteComment(comment.id), words.removed(comment.author_name));
  };
  const reply = async (comment: AdminComment, name: string, text: string) =>
    settle(await replyToComment(comment.id, name, text), words.replied(comment.author_name));

  const comments = listed.data;
  let list = <p role="status">{words.loading}</p>;
  if (comments !== undefined && !isData(comments)) {
    list = <Problem message={problemMessage(comments.problem, en.problems.unreachable)} />;
  } else if (isData(comments) && comments.data.length === 0) {
    list = <p role="status">{words.none[status]}</p>;
  } else if (isData(comments)) {
    list = (
      <ul className="queue">
        {comments.data.map((comment, index) => (
          <CommentCard
            key={comment.id}
            comment={comment}
            first={index === 0 && status === "pending"}
            onApprove={approve}
            onDelete={remove}
            onReply={reply}
          />
        ))}
      </ul>
    );
  }
  return (
    <>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
    </>
  );
}
