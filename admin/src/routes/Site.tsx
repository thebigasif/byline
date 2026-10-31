import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { failureMessage, isData } from "../api/call";
import { changeSite, readSite, type SiteSettings } from "../api/site";
import { Problem } from "../components/Problem";
import { SiteForm } from "../site/SiteForm";
import { type SiteFields, siteChange } from "../site/siteChange";
import { en, type UserMessage } from "../strings/en";

const SITE = ["site"];

/** Settings for the whole blog, saved to `site.toml`. */
export function Site() {
  const words = en.site;
  const queries = useQueryClient();
  const read = useQuery({ queryKey: SITE, queryFn: readSite }).data;
  const [said, setSaid] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  const save = async (site: SiteSettings, fields: SiteFields) => {
    const change = siteChange(site, fields);
    if (change === null) {
      setSaid(words.unchanged);
      return;
    }
    const reply = await changeSite(change);
    setSaid(isData(reply) ? words.saved : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: SITE });
  };

  let shown = <p role="status">{words.loading}</p>;
  if (read !== undefined && !isData(read)) {
    shown = <Problem message={failureMessage(read)} />;
  } else if (isData(read)) {
    const site = read.data;
    shown = (
      <>
        {site.restart_needed ? <p className="careful">{words.restart}</p> : null}
        {failed === null ? null : <Problem message={failed} />}
        <p role="status">{said}</p>
        <SiteForm site={site} onSave={(fields) => save(site, fields)} />
      </>
    );
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {shown}
    </>
  );
}
