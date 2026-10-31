import { en } from "../strings/en";

const words = en.webhooks;
// Looked up by the names the API sends, which may grow.
const KINDS: Readonly<Record<string, string>> = words.kinds;
const HAPPENED: Readonly<Record<string, string>> = words.happened;
const OTHERS: Readonly<Record<string, string>> = words.others;

/** The events a webhook may send, grouped as the menu shows them, from the API's catalogue. */
export const EVENT_GROUPS: { label: string; events: string[] }[] = [
  {
    label: words.groups.content,
    events: Object.keys(KINDS).flatMap((kind) =>
      Object.keys(HAPPENED).map((happened) => `${kind}.${happened}`),
    ),
  },
  { label: words.groups.readers, events: ["comment.added", "subscriber.added", "form.submitted"] },
  {
    label: words.groups.site,
    events: ["tag.added", "lint.blocked", "audit.finished", "site.changed", "alert.raised"],
  },
];

/** An event as a short sentence, such as "A post goes live"; an unknown one by its API name. */
export function eventSentence(name: string): string {
  const other = OTHERS[name];
  if (other !== undefined) {
    return other;
  }
  const dot = name.indexOf(".");
  const kind = KINDS[name.slice(0, dot)];
  const happened = HAPPENED[name.slice(dot + 1)];
  if (dot > 0 && kind !== undefined && happened !== undefined) {
    return `${kind} ${happened}`;
  }
  return name;
}

/** The host an address points at, such as `hooks.zapier.com`, or the address when unreadable. */
export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}
