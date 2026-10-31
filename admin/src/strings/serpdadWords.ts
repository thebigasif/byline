// The words of the Connect to SerpDad page, merged into `en` as `en.serpdad`.
export const serpdadWords = {
  title: "Connect to SerpDad",
  lede: "SerpDad is the SEO tool that makes this blog. When it is connected, SerpDad sends each finished article here as a draft. Khalid checks the draft on arrival, and it waits for you to publish it. Nothing is sent to SerpDad from here.",
  loading: "Reading the connection to SerpDad",
  off: {
    title: "Turn on drafts from SerpDad",
    what: "Turning it on makes a key: a long password just for SerpDad. The key lets SerpDad add drafts and nothing else. It cannot publish, delete posts or change settings.",
    after:
      "You copy the key into SerpDad once. You can turn the connection off at any time, and SerpDad is refused from then on.",
    make: "Make a key for SerpDad",
  },
  key: {
    title: "Copy these two into SerpDad",
    where: 'In SerpDad, open Integrations, choose "Your blog, by SerpDad", and paste them in.',
    address: "Blog address",
    key: "Key",
    once: "You see this key only once.",
    onceWhy:
      "Khalid keeps a scrambled copy that nobody can turn back into the key. If you lose it, make a new one: the old one stops working.",
    copy: "Copy the key",
    copied: "Copied. Paste it into SerpDad now.",
    notCopied: "Your browser did not let Khalid copy it. Select the key and copy it by hand.",
  },
  connected: "Connected",
  since: (date: string) => `since ${date}.`,
  drafts: {
    title: "Drafts received",
    lede: "The last 20 articles SerpDad sent, newest first. A draft with a block cannot be published until you fix it in the editor.",
    none: "SerpDad has not sent a draft yet. Finished articles appear here as they arrive.",
    draft: "Draft",
    checks: "Checks",
    arrived: "Arrived",
    unchecked: "Not checked yet",
  },
  change: {
    title: "Make a new key or turn it off",
    lede: "A new key replaces the old one at once, so paste it into SerpDad straight away. Turning it off refuses SerpDad's next article. Drafts already here stay.",
    renew: "Make a new key",
    off: "Turn off",
    turnedOff: "Turned off. SerpDad's key no longer works.",
  },
} as const;
