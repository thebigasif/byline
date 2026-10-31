import { type FormEvent, useState } from "react";
import { problemMessage } from "../api/posts";
import { addSite, type Site } from "../api/sites";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";

type Props = {
  /** Called with the site the server made, so the list can show it. */
  onAdded: (site: Site) => void;
};

/** The page's one focus: a new site on a domain, and what to do after. */
export function AddSite({ onAdded }: Props) {
  const words = en.sites.add;
  const [domain, setDomain] = useState("");
  const [busy, setBusy] = useState(false);
  const [added, setAdded] = useState<Site | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const wanted = domain.trim().toLowerCase();
    setAdded(null);
    if (wanted === "") {
      setFailed(en.sites.problems.badDomain);
      return;
    }
    if (busy) {
      return;
    }
    setBusy(true);
    const reply = await addSite(wanted);
    setBusy(false);
    if ("data" in reply) {
      setFailed(null);
      setDomain("");
      setAdded(reply.data);
      onAdded(reply.data);
    } else {
      setFailed(problemMessage(reply.problem, en.problems.unreachable));
    }
  };

  return (
    <>
      <section className="focus" aria-labelledby="add-title">
        <h2 id="add-title">{words.title}</h2>
        <p>{words.lede}</p>
        {failed === null ? null : <Problem message={failed} />}
        <form onSubmit={submit} noValidate>
          <div className="field">
            <label htmlFor="site-domain">{words.label}</label>
            <span className="hint" id="site-domain-hint">
              {words.hint}
            </span>
            <input
              id="site-domain"
              autoComplete="off"
              spellCheck={false}
              aria-describedby="site-domain-hint"
              value={domain}
              onChange={(event) => setDomain(event.target.value)}
            />
          </div>
          <button type="submit" className="btn primary" disabled={busy}>
            {words.submit}
          </button>
        </form>
      </section>
      <div role="status">
        {busy ? <p>{words.adding}</p> : null}
        {added === null ? null : (
          <div className="done">
            <p>
              <strong>{words.done(added.domain)}</strong>
            </p>
            <p>{words.dns}</p>
            <p>
              {words.owner} <code>{words.ownerCommand(added.domain)}</code>
            </p>
          </div>
        )}
      </div>
    </>
  );
}
