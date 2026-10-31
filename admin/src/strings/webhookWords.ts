// The Webhooks screen's words, kept beside en.ts so that file stays short.
export const webhookWords = {
  title: "Webhooks",
  lede: "A webhook tells another service when something happens on your blog. When a post goes live, Khalid sends a short message to the address you give, so a tool such as Zapier, Make or Slack can act on it straight away. Each message is signed, so the receiver can check it came from your blog.",
  // An event name is a kind and what happened to it, such as `post.published`.
  kinds: {
    post: "A post",
    page: "A page",
    "glossary-term": "A glossary term",
    "case-study": "A case study",
    "changelog-entry": "A changelog entry",
  },
  happened: {
    published: "goes live",
    "published.edited": "is edited while live",
    unpublished: "is taken down",
    scheduled: "is scheduled",
    deleted: "is deleted",
  },
  others: {
    "tag.added": "A tag is added",
    "comment.added": "A reader writes a comment",
    "subscriber.added": "Someone subscribes",
    "form.submitted": "Someone sends a form",
    "lint.blocked": "The checks stop a post",
    "audit.finished": "A site audit finishes",
    "site.changed": "The settings change",
    "alert.raised": "An alert is raised",
  },
  groups: { content: "Posts and pages", readers: "Readers", site: "The site" },
  add: {
    title: "Add a webhook",
    event: "When",
    eventHint: "The one thing that sends a message. Add another webhook for each other thing.",
    url: "Send to",
    urlHint: "The full address the service gave you, starting with https://.",
    once: "Khalid shows the secret once, when the webhook is added. If a message fails, Khalid tries again later, several times.",
    submit: "Add webhook",
  },
  shown: {
    title: (event: string, host: string) =>
      `Your new webhook sends ${event} to ${host}. Copy its secret now: it is not shown again.`,
    label: "The secret",
    hint: "Paste it into the receiving service, which uses it to check each message.",
  },
  list: {
    title: "Webhooks that are on",
    lede: "Switching a webhook off works at once: nothing more is sent to it.",
    loading: "Reading your webhooks",
    none: "No webhooks yet. Add one above to tell another service when something happens.",
    event: "When",
    url: "Sends to",
    added: "Added",
    action: "Action",
    remove: "Switch off",
    removeLabel: (event: string, host: string) => `Switch off the webhook for ${event} to ${host}`,
    removed: (event: string, host: string) =>
      `Switched off: ${event}, to ${host}. Nothing more is sent.`,
  },
} as const;
