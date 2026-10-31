import type { Site } from "../api/sites";
import { en } from "../strings/en";

type Props = { sites: readonly Site[] };

/** Every site this server runs, the home site first and marked. */
export function SiteTable({ sites }: Props) {
  const words = en.sites.list;
  return (
    <section aria-labelledby="sites-title">
      <h2 id="sites-title">{words.title}</h2>
      <p className="muted">{words.lede}</p>
      <table>
        <thead>
          <tr>
            <th scope="col">{words.domain}</th>
            <th scope="col">{words.kind}</th>
          </tr>
        </thead>
        <tbody>
          {sites.map((site) => (
            <tr key={site.domain}>
              <td data-label={words.domain}>
                <a href={`https://${site.domain}/`}>{site.domain}</a>
              </td>
              <td data-label={words.kind}>
                {site.home ? <span className="pill quiet">{words.home}</span> : null}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
