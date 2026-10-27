// Every word the admin shows, so a second language is one more file.
export const en = {
  brand: "Khalid",
  skip: "Skip to the main content",
  viewBlog: "View the blog",
  nav: {
    label: "Admin",
    home: "Home",
  },
  home: {
    title: "Your blog's admin",
    lede: "Write, check and publish posts here. Each one is saved as a text file on the server.",
  },
  problems: {
    page: {
      what: "This screen stopped working.",
      why: "Something went wrong while showing it. Nothing you saved is lost.",
      fix: "Reload the page. If it happens again, sign out and back in.",
    },
  },
} as const;

/** What happened, why it matters, and what to do, each one sentence. */
export type UserMessage = { readonly what: string; readonly why: string; readonly fix: string };
