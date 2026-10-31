// The words of the Sites page, where a super admin of a server with many
// sites sees them and adds one. en.ts carries them as en.sites; they sit
// in their own file so en.ts stays short.
export const sitesWords = {
  title: "Sites on this server",
  lede: "This server runs several blogs, each on its own domain, with its own posts, staff and design. As a super admin you can add one here. It answers at once, with no restart.",
  openAdmin: "Open this site's admin",
  loading: "Reading the sites on this server",
  add: {
    title: "Add a site",
    lede: "Khalid makes the new site its own folder and database on this server. Nothing on the other sites changes.",
    label: "Domain",
    hint: "The web address readers type, without https://, such as news.example.com.",
    submit: "Add the site",
    adding: "Adding the site",
    done: (domain: string) => `Added ${domain}. Khalid serves it now.`,
    dns: "Next, point the domain's DNS at this server, the way you did for the home site.",
    owner: "Then give it an owner on the server:",
    ownerCommand: (domain: string) =>
      `khalid --site sites/${domain} staff owner --email you@example.com --name "Your Name"`,
  },
  list: {
    title: "The sites",
    lede: "The home site comes first. It is where super admins sign in and where this page lives.",
    domain: "Domain",
    kind: "Kind",
    home: "Home site",
  },
  signIn: {
    title: "Sign in as yourself first",
    lede: "Adding a site gives it a domain on this server, so the page needs you, not a key. Khalid emails you a link that signs you in.",
    label: "Your email",
    hint: "The address khalid sites admin named as a super admin.",
    submit: "Email me a sign-in link",
    sending: "Sending the link",
    sent: (email: string) =>
      `If ${email} has an account here, a sign-in link is on its way. It works once, within 15 minutes.`,
    noMail: "No email arrives? The server may have no mail set up yet. Ask whoever runs it for a sign-in link.",
  },
  link: {
    title: "Signing you in",
    checking: "Checking the sign-in link",
    back: "Ask for a new link",
  },
  problems: {
    badDomain: {
      what: "The domain is empty.",
      why: "Each site answers on its own domain, so Khalid needs one to add it.",
      fix: "Write the domain readers will type, such as news.example.com.",
    },
    badLink: {
      what: "This sign-in link did not work.",
      why: "A link works once and only for 15 minutes, so it may be used or old.",
      fix: "Ask for a new link and open it soon after it arrives.",
    },
  },
} as const;
