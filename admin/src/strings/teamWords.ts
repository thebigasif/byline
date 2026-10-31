// The Team screen's words, kept beside en.ts so that file stays short.
export const teamWords = {
  title: "Team",
  lede: "Your team is everyone who can sign in to this admin. Each person has a role, and the role decides what they may do. You can give someone only a role below your own, so nobody hands out more than they hold.",
  roles: {
    owner: "Owner",
    admin: "Admin",
    editor: "Editor",
    writer: "Writer",
    contributor: "Contributor",
  },
  roleHints: {
    writer: "Writes, publishes and uploads for their own posts.",
    contributor: "Writes drafts. Someone else publishes them.",
    editor:
      "Edits and publishes anyone's posts, answers comments, and manages writers and contributors.",
    admin:
      "Everything, including settings, keys and the team. Only the owner can hand the site to someone else.",
  },
  invite: {
    title: "Invite someone",
    name: "Name",
    nameHint: "As the admin shows it.",
    email: "Email",
    emailHint: "They sign in with it.",
    role: "Role",
    noEmail:
      "No email is sent now. Tell them to open this admin and sign in with that email address: Khalid sends a sign-in link, and they choose a password or a passkey there.",
    submit: "Invite",
    invited: (name: string) => `${name} is on the team. Tell them to sign in with their email.`,
  },
  list: {
    title: "Your team",
    lede: "A new role works from their next request. Removing someone signs them out on every device.",
    loading: "Reading your team",
    name: "Name",
    email: "Email",
    role: "Role",
    action: "Action",
    roleFor: (name: string) => `Role for ${name}`,
    change: "Change role",
    changeLabel: (name: string) => `Change the role of ${name}`,
    changed: (name: string, role: string) => `${name}'s role is now ${role}.`,
    owner: "Nobody can change the owner",
    remove: "Remove",
    removeLabel: (name: string) => `Remove ${name}`,
    removed: (name: string) => `${name} is off the team and signed out everywhere.`,
  },
} as const;
