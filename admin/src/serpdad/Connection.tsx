import type { SerpdadState } from "../api/serpdad";
import { day } from "../strings/dates";
import { en } from "../strings/en";
import { ChangeKey } from "./ChangeKey";
import { DraftsTable } from "./DraftsTable";
import { KeyShown } from "./KeyShown";
import { TurnOn } from "./TurnOn";

type Props = {
  state: SerpdadState;
  secret: string | null;
  busy: boolean;
  onMake: () => void;
  onOff: () => void;
};

/** One focus at a time: turn it on, copy the new key, or see the drafts while connected. */
export function Connection({ state, secret, busy, onMake, onOff }: Props) {
  const words = en.serpdad;
  const connection = state.connection ?? null;
  if (connection === null) {
    return (
      <>
        <TurnOn busy={busy} onMake={onMake} />
        {state.drafts.length === 0 ? null : <DraftsTable drafts={state.drafts} />}
      </>
    );
  }
  return (
    <>
      {secret === null ? null : <KeyShown secret={secret} />}
      <p>
        <span className="pill ok">{words.connected}</span>{" "}
        {words.since(day(connection.connected_at))}
      </p>
      <DraftsTable drafts={state.drafts} />
      <ChangeKey busy={busy} onRenew={onMake} onOff={onOff} />
    </>
  );
}
