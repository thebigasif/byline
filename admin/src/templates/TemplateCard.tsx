import { Fragment } from "react";
import type { SchemaTemplate } from "../api/schemaTemplates";
import { en } from "../strings/en";

type Props = { template: SchemaTemplate; at: number; busy: boolean; onRemove: () => void };

/** One template in use: which posts get which kind of node, and its details. */
export function TemplateCard({ template, at, busy, onRemove }: Props) {
  const words = en.templates;
  const id = `template-${at}`;
  const properties = Object.entries(template.properties ?? {});
  return (
    <section className="card template" aria-labelledby={id}>
      <h3 id={id}>{words.every(template.applies_to, template.name, template.type)}</h3>
      {properties.length === 0 ? null : (
        <dl>
          {properties.map(([name, value]) => (
            <Fragment key={name}>
              <dt>{name}</dt>
              <dd>{value}</dd>
            </Fragment>
          ))}
        </dl>
      )}
      <button type="button" className="btn" onClick={onRemove} disabled={busy}>
        {words.remove}
      </button>
    </section>
  );
}
