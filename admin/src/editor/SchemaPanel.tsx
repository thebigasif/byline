import type { PostCheck } from "../api/check";
import { en } from "../strings/en";
import { SettingField } from "./SettingField";
import type { SettingsDraft, TextSetting } from "./settings";
import { richResultsTest, schemaTypes } from "./snippet";

type Props = {
  /** The saved post's check, or null before the blog has checked it. */
  check: PostCheck | null;
  /** Whether the post is live, so Google's test can read its page. */
  live: boolean;
  settings: SettingsDraft;
  onSetting: (key: TextSetting, value: string) => void;
  onBlur: () => void;
};

/** The schema the page carries, a test for it, and the writer's own JSON-LD. */
export function SchemaPanel({ check, live, settings, onSetting, onBlur }: Props) {
  const words = en.schema;
  return (
    <details>
      <summary>{words.title}</summary>
      <div>
        <p className="hint">{words.lede}</p>
        {check === null ? null : (
          <>
            <ul className="types" aria-label={words.types}>
              {schemaTypes(check.schema).map((kind) => (
                <li key={kind}>{kind}</li>
              ))}
            </ul>
            <details>
              <summary>{words.show}</summary>
              <div>
                <pre className="json">{JSON.stringify(check.schema, null, 2)}</pre>
              </div>
            </details>
            {live ? (
              <p>
                <a href={richResultsTest(check.snippet.url)}>{words.test}</a>
              </p>
            ) : null}
          </>
        )}
        <SettingField id="setting-schema" label={words.own} hint={words.ownHint}>
          <textarea
            id="setting-schema"
            className="source"
            aria-describedby="setting-schema-hint"
            value={settings.schema}
            onChange={(event) => onSetting("schema", event.target.value)}
            onBlur={onBlur}
            rows={6}
            spellCheck={false}
          />
        </SettingField>
      </div>
    </details>
  );
}
