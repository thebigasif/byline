// The words of the Passkeys screen. The admin's other words are in en.ts.

export const passkeys = {
  title: "Passkeys",
  lede: "A passkey lets you sign in with your fingerprint, your face or your device's PIN, instead of a password. Your phone or laptop keeps it, and it works only on this blog, so nobody can trick you into using it on a fake page.",
  loading: "Reading your passkeys",
  addTitle: "Add a passkey on this device",
  addLede:
    "Your browser will ask you to unlock this device. After that, you can sign in here with this device alone.",
  nameHint: "So you can tell your passkeys apart, such as the device it lives on.",
  addText: "Add passkey",
  adding: "Waiting for this device",
  madeOne: (name: string) => `${name} is added. You can sign in with it now.`,
  listTitle: "Your passkeys",
  listHint:
    "Remove a passkey when you lose or give away the device it lives on. Removing one works at once. Your password and email sign-in links keep working.",
  none: "You have no passkeys yet. Add one above to sign in without a password.",
  count: (n: number) => (n === 1 ? "You have 1 passkey." : `You have ${n} passkeys.`),
  name: "Name",
  added: "Added",
  lastUsed: "Last used",
  never: "Not used yet",
  action: "What to do",
  renameText: "Rename",
  rename: (name: string) => `Rename ${name}`,
  newName: (name: string) => `New name for ${name}`,
  saveName: "Save name",
  keepName: "Keep the old name",
  renamed: (name: string) => `The passkey is now called ${name}.`,
  removeText: "Remove",
  remove: (name: string) => `Remove ${name}`,
  asking: (name: string) => `Remove ${name}? It will no longer sign you in.`,
  sureText: "Yes, remove it",
  sure: (name: string) => `Yes, remove ${name}`,
  keep: "Keep it",
  removed: (name: string) => `${name} is removed. It no longer signs you in.`,
  problems: {
    noSession: {
      what: "This tab signed in with a key, not as a person.",
      why: "A passkey belongs to a person's account, and a key belongs to none.",
      fix: "Sign out, then sign in with your email to manage your passkeys.",
    },
    noPasskey: {
      what: "This device did not make a passkey.",
      why: "It was cancelled, it timed out, or this browser cannot make passkeys.",
      fix: "Press Add passkey again and unlock the device when asked, or try another browser.",
    },
    blankName: {
      what: "The passkey needs a name.",
      why: "The name tells your passkeys apart in this list.",
      fix: "Type a name of up to 60 characters, such as Work laptop.",
    },
  },
} as const;
