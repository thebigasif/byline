import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { problemMessage } from "../api/posts";
import {
  readTemplates,
  type SchemaTemplate,
  saveTemplates,
  type SchemaTemplates as Templates,
} from "../api/schemaTemplates";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { TemplateCard } from "../templates/TemplateCard";
import { TemplateForm } from "../templates/TemplateForm";

/** The schema every post in a collection gets: the templates in use, and a form to add one. */
export function SchemaTemplates() {
  const words = en.templates;
  const read = useQuery({ queryKey: ["schema-templates"], queryFn: readTemplates });
  const [saved, setSaved] = useState<Templates | null>(null);
  const [problem, setProblem] = useState<UserMessage | null>(null);
  const [busy, setBusy] = useState(false);

  if (read.isPending) {
    return <p role="status">{words.loading}</p>;
  }
  const answer = read.data;
  const held = answer !== undefined && "templates" in answer ? answer.templates : null;
  const current = saved ?? held;
  if (current === null) {
    const refusal = answer !== undefined && "problem" in answer ? answer.problem : null;
    return (
      <>
        <h1>{words.title}</h1>
        <Problem message={problemMessage(refusal, en.problems.unreachable)} />
      </>
    );
  }
  const save = async (templates: SchemaTemplate[]) => {
    setBusy(true);
    const sent = await saveTemplates(templates, current.updated_at);
    setBusy(false);
    if ("templates" in sent) {
      setSaved(sent.templates);
      setProblem(null);
      return true;
    }
    setProblem(problemMessage(sent.problem, en.problems.unreachable));
    return false;
  };
  const without = (at: number) => current.templates.filter((_, each) => each !== at);

  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      {current.restart_needed ? <p className="saved">{words.restart}</p> : null}
      {problem === null ? null : <Problem message={problem} />}
      <h2>{words.inUse}</h2>
      {current.templates.length === 0 ? <p className="hint">{words.none}</p> : null}
      {current.templates.map((template, at) => (
        <TemplateCard
          key={`${template.applies_to} ${template.name} ${template.type}`}
          template={template}
          at={at}
          busy={busy}
          onRemove={() => void save(without(at))}
        />
      ))}
      <TemplateForm
        collections={current.collections}
        busy={busy}
        onAdd={(template) => save([...current.templates, template])}
      />
    </>
  );
}
