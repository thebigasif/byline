import { type FormEvent, useState } from "react";
import type { SchemaTemplate } from "../api/schemaTemplates";
import { SettingField } from "../editor/SettingField";
import { en } from "../strings/en";

type Props = {
  collections: string[];
  busy: boolean;
  /** Saves the new template beside the others; true once it is saved. */
  onAdd: (template: SchemaTemplate) => Promise<boolean>;
};

/** One detail of the node: a schema.org property name and its value. */
type Detail = { id: number; name: string; value: string };

/** The form that adds a schema template for one collection. */
export function TemplateForm({ collections, busy, onAdd }: Props) {
  const words = en.templates.form;
  const [collection, setCollection] = useState(collections[0] ?? "posts");
  const [kind, setKind] = useState("");
  const [details, setDetails] = useState<Detail[]>([{ id: 0, name: "", value: "" }]);
  const setDetail = (id: number, change: Partial<Detail>) => {
    setDetails((all) => all.map((each) => (each.id === id ? { ...each, ...change } : each)));
  };
  const addDetail = () => {
    setDetails((all) => [...all, { id: all.length, name: "", value: "" }]);
  };
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const named = details.filter((detail) => detail.name.trim() !== "");
    const properties = Object.fromEntries(named.map((each) => [each.name.trim(), each.value]));
    const template = { applies_to: "collection" as const, name: collection, type: kind.trim() };
    if (await onAdd({ ...template, properties })) {
      setKind("");
      setDetails([{ id: 0, name: "", value: "" }]);
    }
  };

  return (
    <form onSubmit={(event) => void submit(event)} aria-labelledby="template-add">
      <h2 id="template-add">{words.title}</h2>
      <SettingField id="template-collection" label={words.collection} hint={words.collectionHint}>
        <select
          id="template-collection"
          aria-describedby="template-collection-hint"
          value={collection}
          onChange={(event) => setCollection(event.target.value)}
        >
          {collections.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      </SettingField>
      <SettingField id="template-type" label={words.type} hint={words.typeHint}>
        <input
          id="template-type"
          aria-describedby="template-type-hint"
          value={kind}
          onChange={(event) => setKind(event.target.value)}
          required
          spellCheck={false}
        />
      </SettingField>
      <fieldset>
        <legend>{words.details}</legend>
        <p className="hint">{words.detailsHint}</p>
        {details.map((detail) => (
          <div className="pair" key={detail.id}>
            <div className="field">
              <label htmlFor={`detail-name-${detail.id}`}>{words.name}</label>
              <input
                id={`detail-name-${detail.id}`}
                value={detail.name}
                onChange={(event) => setDetail(detail.id, { name: event.target.value })}
                spellCheck={false}
              />
            </div>
            <div className="field">
              <label htmlFor={`detail-value-${detail.id}`}>{words.value}</label>
              <input
                id={`detail-value-${detail.id}`}
                value={detail.value}
                onChange={(event) => setDetail(detail.id, { value: event.target.value })}
              />
            </div>
          </div>
        ))}
        <button type="button" className="btn" onClick={addDetail}>
          {words.more}
        </button>
      </fieldset>
      <button type="submit" className="btn primary" disabled={busy}>
        {busy ? words.saving : words.save}
      </button>
    </form>
  );
}
