// Every word the admin shows, so a second language is one more file.
export const en = {
  brand: "Khalid",
  skip: "Skip to the main content",
  viewBlog: "View the blog",
  signOut: "Sign out",
  // Dates are written the way this language writes them.
  locale: "en-GB",
  nav: {
    label: "Admin",
    dashboard: "Dashboard",
    posts: "Posts",
  },
  dashboard: {
    title: "Your blog today",
    loading: "Reading your posts",
    calm: "Nothing is blocked from publishing. Start a new post, or pick up a recent one below.",
    blockedLede: (count: number) =>
      count === 1
        ? "One post has a problem that stops it from publishing."
        : `${count} posts have a problem that stops them from publishing.`,
    blockedTitle: (count: number) =>
      count === 1 ? "1 post cannot go live" : `${count} posts cannot go live`,
    blockedWhy:
      "Khalid's checks found a problem in each one. Readers see none of them until it is fixed.",
    blockedFix: "Open a post. Its checks say what to change.",
    showBlocked: "Show every blocked post",
    glance: "At a glance",
    live: (count: number) => (count === 1 ? "post live on the blog" : "posts live on the blog"),
    scheduled: (count: number) =>
      count === 1 ? "post scheduled to go live" : "posts scheduled to go live",
    drafts: (count: number) => (count === 1 ? "draft" : "drafts"),
    blocked: (count: number) =>
      count === 1 ? "post blocked from publishing" : "posts blocked from publishing",
    recent: "Recent changes",
    noPosts: "No posts yet. The first one you start shows here.",
    post: "Post",
    stands: "Where it stands",
    who: "Who",
    when: "When",
    nobody: "Not recorded",
  },
  status: {
    draft: "Draft",
    published: "Live",
    scheduled: "Scheduled",
  },
  posts: {
    title: "Posts",
    lede: "Every post on the blog, the latest change first. Pick a title to open it in the editor.",
    write: "Write a new post",
    loading: "Reading your posts",
    found: (count: number) => (count === 1 ? "1 post matches." : `${count} posts match.`),
    none: "No post matches. Change the filters, or write a new post.",
    titleColumn: "Title",
    author: "Author",
    noAuthor: "No author yet",
    checks: "Checks",
    review: "Review",
    index: "Search engines",
    changed: "Last change",
    clear: "No problems",
    blocks: (count: number) => (count === 1 ? "1 block" : `${count} blocks`),
    warnings: (count: number) => (count === 1 ? "1 warning" : `${count} warnings`),
    terms: [
      {
        term: "Checks",
        meaning:
          "Khalid reads every post for problems before it goes live. A block stops publishing. A warning does not.",
      },
      {
        term: "Review",
        meaning:
          "Where the post is on its way to readers: draft, in review, ready, scheduled or live.",
      },
      {
        term: "Search engines",
        meaning:
          "Whether this blog lets search engines such as Google list the post. Google's own answer comes once Search Console is connected.",
      },
    ],
  },
  filters: {
    label: "Filter posts",
    anyChecks: "Any result",
    anyStage: "Any stage",
    anyIndex: "Any answer",
    checks: {
      blocked: "Blocked from publishing",
      clear: "Nothing blocks it",
    },
    show: "Show posts",
  },
  paging: {
    label: "Pages",
    previous: "Previous page",
    next: "Next page",
    where: (page: number, pages: number) => `Page ${page} of ${pages}`,
  },
  stage: {
    draft: "Draft",
    in_review: "In review",
    ready: "Ready",
    scheduled: "Scheduled",
    live: "Live",
  },
  index: {
    indexable: "May be listed",
    noindex: "Kept out",
    canonical_elsewhere: "Credited to another address",
    not_live: "Not live yet",
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
    publish: "Publish",
    publishing: "Publishing",
    published: "Published",
    blocked: "Fix what stops publishing, then publish again.",
    checks: "Checks",
    noChecks:
      "Khalid checks the post when you publish. Anything that stops it, or could make it better, shows here.",
    blockTag: "Stops publishing",
    warnTag: "Warning, you can still publish",
    here: "Here:",
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
