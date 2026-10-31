import { useEffect } from "react";
import { isData } from "../../api/redirects";
import { useSnippets } from "../../api/snippets";
import { en } from "../../strings/en";

type Props = {
  id: string;
  value: string;
  onChange: (value: string) => void;
};

/** A list of the site's snippets by name, for the snippet block's form. */
export function SnippetChoice({ id, value, onChange }: Props) {
  const reply = useSnippets();
  const found = isData(reply) ? reply.data : [];
  const chosen = value === "" ? (found[0]?.slug ?? "") : value;
  // The first snippet is chosen until the writer picks another.
  useEffect(() => {
    if (chosen !== value) {
      onChange(chosen);
    }
  }, [chosen, value, onChange]);
  if (isData(reply) && found.length === 0) {
    return (
      <p className="hint" id={id}>
        {en.snippets.none}
      </p>
    );
  }
  return (
    <select
      id={id}
      value={chosen}
      disabled={!isData(reply)}
      onChange={(event) => onChange(event.target.value)}
    >
      {found.map((snippet) => (
        <option key={snippet.id} value={snippet.slug}>
          {snippet.title}
        </option>
      ))}
    </select>
  );
}
