import { type FormEvent, useState } from "react";
import type { Invite } from "../api/staff";
import { en } from "../strings/en";
import { GIVEN } from "./roles";

type Props = { onInvite: (invite: Invite) => Promise<boolean> };

/** A new person's name, email and role, each role explained before it is picked. */
export function InviteForm({ onInvite }: Props) {
  const words = en.team.invite;
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<string>("writer");
  const [busy, setBusy] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    const invited = await onInvite({ name: name.trim(), email: email.trim(), role });
    setBusy(false);
    if (invited) {
      setName("");
      setEmail("");
    }
  };

  return (
    <form onSubmit={submit}>
      <div className="field">
        <label htmlFor="invite-name">{words.name}</label>
        <span className="hint" id="invite-name-hint">
          {words.nameHint}
        </span>
        <input
          id="invite-name"
          value={name}
          required
          aria-describedby="invite-name-hint"
          onChange={(event) => setName(event.target.value)}
        />
      </div>
      <div className="field">
        <label htmlFor="invite-email">{words.email}</label>
        <span className="hint" id="invite-email-hint">
          {words.emailHint}
        </span>
        <input
          id="invite-email"
          type="email"
          value={email}
          required
          autoComplete="off"
          aria-describedby="invite-email-hint"
          onChange={(event) => setEmail(event.target.value)}
        />
      </div>
      <fieldset>
        <legend>{words.role}</legend>
        {GIVEN.map((each) => (
          <label className="choice" key={each}>
            <input
              type="radio"
              name="invite-role"
              checked={role === each}
              onChange={() => setRole(each)}
            />
            <span>
              <strong>{en.team.roles[each]}</strong>
              <span className="hint">{en.team.roleHints[each]}</span>
            </span>
          </label>
        ))}
      </fieldset>
      <p className="muted">{words.noEmail}</p>
      <button type="submit" className="btn primary" disabled={busy}>
        {words.submit}
      </button>
    </form>
  );
}
