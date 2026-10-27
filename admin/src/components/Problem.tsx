import type { UserMessage } from "../strings/en";

type Props = { message: UserMessage };

/** Shows what went wrong, why it matters and what to do, in that order. */
export function Problem({ message }: Props) {
  return (
    <div className="problem" role="alert">
      <p>
        <strong>{message.what}</strong>
      </p>
      <p>{message.why}</p>
      <p>{message.fix}</p>
    </div>
  );
}
