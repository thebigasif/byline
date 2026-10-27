// Every word the admin shows, so a second language is one more file.
export const en = {
  brand: "Khalid",
  skip: "Skip to the main content",
  viewBlog: "View the blog",
  signOut: "Sign out",
  nav: {
    label: "Admin",
    home: "Home",
  },
  home: {
    title: "Your blog's admin",
    lede: "Write, check and publish posts here. Each one is saved as a text file on the server.",
  },
  start: {
    title: "Start a post",
    lede: "Give it a working title. It is saved as a draft at once, and nobody sees it until you publish.",
    label: "Title",
    submit: "Start writing",
    starting: "Making the draft",
  },
  editor: {
    heading: "Editing",
    loading: "Opening the post",
    title: "Title",
    text: "Post text",
    modes: "How to write",
    rich: "Rich text",
    markdown: "Markdown",
    onlyMarkdown:
      "This post uses blocks the rich-text view cannot show yet, so it stays in markdown. Your blocks stay exactly as written.",
    saved: "Saved",
    saving: "Saving",
    unsaved: "Not saved yet",
    leave: "Some changes are not saved yet. Leaving now loses them.",
    stay: "Stay on this post",
    leaveAnyway: "Leave and lose them",
  },
  signIn: {
    title: "Sign in to your blog's admin",
    lede: "The admin works with a staff token, a key that lets one person edit this blog.",
    label: "Staff token",
    hint: "Make one on the server with khalid keys create --kind staff --role editor --name followed by your name. It is shown once.",
    submit: "Sign in",
    checking: "Checking the token",
  },
  problems: {
    rejected: {
      what: "The blog did not accept this token.",
      why: "It may be mistyped, revoked, or made for another blog.",
      fix: "Paste it again, or make a new one with khalid keys create.",
    },
    lacksScope: {
      what: "This key cannot edit posts.",
      why: "The admin needs a key that may write posts, and this one may not.",
      fix: "Make a staff token with the editor role, or an integration key with posts:write.",
    },
    unreachable: {
      what: "The admin could not reach the blog's server.",
      why: "Without the server, nothing can be read or saved.",
      fix: "Check that khalid serve is running, then try again.",
    },
    notFound: {
      what: "This post does not exist.",
      why: "It may have been deleted, or the address is mistyped.",
      fix: "Go back home and open the post again.",
    },
    notSaved: {
      what: "The last change was not saved.",
      why: "The blog did not answer, so the change is only in this tab.",
      fix: "Keep this tab open; the admin tries again with your next change.",
    },
    page: {
      what: "This screen stopped working.",
      why: "Something went wrong while showing it. Nothing you saved is lost.",
      fix: "Reload the page. If it happens again, sign out and back in.",
    },
  },
} as const;

/** What happened, why it matters, and what to do, each one sentence. */
export type UserMessage = { readonly what: string; readonly why: string; readonly fix: string };
