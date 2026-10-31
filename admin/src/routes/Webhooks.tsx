import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { failureMessage, isData } from "../api/call";
import {
  type AdminWebhook,
  addWebhook,
  listWebhooks,
  type NewWebhook,
  removeWebhook,
} from "../api/webhooks";
import { Problem } from "../components/Problem";
import { en, type UserMessage } from "../strings/en";
import { eventSentence, hostOf } from "../webhooks/events";
import { NewWebhookForm } from "../webhooks/NewWebhookForm";
import { SecretOnce } from "../webhooks/SecretOnce";
import { WebhookTable } from "../webhooks/WebhookTable";

/** The query key the screen refreshes after a webhook is added or switched off. */
export const WEBHOOKS = ["webhooks"];

/** The webhooks that tell other services what happens on the blog. */
export function Webhooks() {
  const words = en.webhooks;
  const queries = useQueryClient();
  const listed = useQuery({ queryKey: WEBHOOKS, queryFn: listWebhooks }).data;
  const [done, setDone] = useState<string | null>(null);
  const [failed, setFailed] = useState<UserMessage | null>(null);

  // The secret lives only in this screen's state: gone on leaving or reloading.
  const [added, setAdded] = useState<AdminWebhook | null>(null);
  const [refused, setRefused] = useState<UserMessage | null>(null);

  const add = async (webhook: NewWebhook) => {
    const reply = await addWebhook(webhook);
    setAdded(isData(reply) ? (reply.data[0] ?? null) : null);
    setRefused(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: WEBHOOKS });
    return isData(reply);
  };

  const remove = async (webhook: AdminWebhook) => {
    const reply = await removeWebhook(webhook.id);
    const named = words.list.removed(eventSentence(webhook.event), hostOf(webhook.url));
    setDone(isData(reply) ? named : null);
    setFailed(isData(reply) ? null : failureMessage(reply));
    await queries.invalidateQueries({ queryKey: WEBHOOKS });
  };

  let list = <p role="status">{words.list.loading}</p>;
  if (listed !== undefined && !isData(listed)) {
    list = <Problem message={failureMessage(listed)} />;
  } else if (isData(listed)) {
    list = <WebhookTable webhooks={listed.data} onRemove={remove} />;
  }
  return (
    <>
      <h1>{words.title}</h1>
      <p className="lede">{words.lede}</p>
      <section className="focus" aria-labelledby="new-webhook-title">
        <h2 id="new-webhook-title">{words.add.title}</h2>
        {refused === null ? null : <Problem message={refused} />}
        {added === null ? null : <SecretOnce added={added} />}
        <NewWebhookForm onAdd={add} />
      </section>
      <h2 id="webhooks-title">{words.list.title}</h2>
      <p className="muted">{words.list.lede}</p>
      {failed === null ? null : <Problem message={failed} />}
      <p role="status">{done}</p>
      {list}
    </>
  );
}
