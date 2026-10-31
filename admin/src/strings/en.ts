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
    redirects: "Redirects",
  },
  redirects: {
    title: "Redirects",
    lede: "A redirect sends visitors and Google from an old web address to a new one. When you rename a post, a redirect keeps old links working and keeps the post's place in search results.",
    send: {
      title: "Send an old address to a post",
      lede: "Khalid writes the old address into the post's file, among its old addresses. The redirect then moves with the post, and git keeps a record of it.",
      from: "Old address",
      fromHint: "It must start with /, such as /2019/05/old-title/.",
      to: "Post it should open",
      toHint: "Visitors land here instead.",
      pick: "Pick a post",
      permanent:
        "This is a permanent move, the kind Google follows and remembers (a 301 redirect).",
      submit: "Add redirect",
      adding: "Adding the redirect",
      loading: "Reading your posts",
      noPosts: "No post is live yet. Once one is, you can send old addresses to it here.",
      done: (from: string) => `Added. ${from} now opens`,
      where: "You find it in that post under More settings, Old addresses.",
    },
    misses: {
      title: "Addresses that found nothing",
      lede: 'These addresses on your blog answered "page not found" (a 404). Visitors and Google gave up on them. Send each one to the page Khalid suggests, or leave it.',
      loading: "Reading the addresses that found nothing",
      none: "No address on your blog has answered page not found yet.",
      path: "Address",
      hits: "Visits",
      last: "Last visit",
      nearest: "Suggested page",
      action: "Action",
      noneNear: "None near enough to guess",
      send: (path: string) => `Redirect it there: ${path}`,
      sendText: "Redirect it there",
      sent: (path: string, to: string) => `Done. ${path} now opens ${to}.`,
    },
    kept: {
      title: "Redirects Khalid keeps",
      lede: "Redirects you imported, made from the list above, or that Khalid made when a post's address changed. Khalid follows chains for you: if an old address points to another old address, visitors still go straight to the page in one step.",
      after:
        "Old addresses kept in a post's own file are not listed here. Each post shows them under More settings.",
      loading: "Reading your redirects",
      none: "Khalid keeps no redirects yet. Old addresses you add to a post are kept in its file.",
      from: "Old address",
      to: "Opens",
      kind: "Kind",
      origin: "Made by",
      action: "Action",
      nothing: "Nothing",
      remove: (from: string) => `Remove the redirect from ${from}`,
      removeText: "Remove",
      removed: (from: string) => `Removed. ${from} no longer redirects.`,
    },
    csv: {
      title: "Many redirects at once",
      lede: "A CSV file is a spreadsheet saved as plain text. Each row is one redirect, with the columns from, to, status and match. Khalid stores the whole file or nothing, so a mistake in one row never leaves half a list.",
      download: "Download every redirect as CSV",
      file: "Add redirects from a CSV file",
      fileHint: "From a spreadsheet, another blog or the download above.",
      add: "Add these redirects",
      adding: "Adding the redirects",
      added: (count: number) => (count === 1 ? "Added 1 redirect." : `Added ${count} redirects.`),
    },
    // What a visitor gets, by the status the redirect answers.
    kind: (status: number, pattern: boolean) => {
      const kinds: Record<number, string> = {
        301: "Moved for good (301)",
        302: "Moved for now (302)",
        307: "Moved for now (307)",
        410: "Gone for good (410)",
        451: "Removed for legal reasons (451)",
      };
      const kind = kinds[status] ?? `Answers ${status}`;
      return pattern ? `${kind}, a pattern` : kind;
    },
    origins: {
      manual: "You",
      import: "Imported",
      slug_change: "A post's new address",
    },
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
  settings: {
    title: "Post settings",
    slug: "Web address",
    slugHint:
      "The last part of the post's address, in lowercase words joined by hyphens. Settle it before the post goes live, so links to it keep working.",
    author: "Author",
    authorHint:
      "The author's short name, as in the file name in your blog's authors folder, such as asif.",
    coAuthors: "Other authors",
    coAuthorsHint: "Their short names, separated by commas. Readers see them after the author.",
    date: "Date shown to readers",
    dateHint: "The day printed on the post. Leave it empty to use the day it goes live.",
    excerpt: "Description",
    excerptHint: (count: number) =>
      `The line under the title in search results, on cards and in feeds. ${count} of 160 characters.`,
    review: "Review",
    reviewHint: "Where the post is on its way to readers. Ready means someone has checked it.",
    more: {
      title: "More settings",
      groups: {
        image: "Feature image",
        search: "Search engines and old addresses",
        x: "Share card on X",
        facebook: "Share card on Facebook",
        down: "Taking it down",
      },
      fields: {
        imageSrc: {
          label: "Image",
          hint: "The picture at the top of the post and on its cards, such as /media/cluster.jpg.",
        },
        imageAlt: {
          label: "What the image shows",
          hint: "Read aloud to readers who cannot see it. The image needs it.",
        },
        imageCaption: {
          label: "Caption",
          hint: "The line under the image. Leave it empty for none.",
        },
        canonical: {
          label: "Canonical address",
          hint: "If this post first appeared elsewhere, that address, so search engines credit it.",
        },
        robots: {
          label: "Search engine visibility",
          hint: "Whether search engines may list this post and follow its links.",
        },
        aliases: {
          label: "Old addresses",
          hint: "Addresses on this blog that should lead here, one per line, such as /2019/05/old-title/.",
        },
        xTitle: { label: "Headline on X", hint: "Leave it empty to use the title." },
        xDescription: { label: "Line on X", hint: "Leave it empty to use the description." },
        xImage: { label: "Image on X", hint: "Leave it empty to use the feature image." },
        facebookTitle: { label: "Headline on Facebook", hint: "Leave it empty to use the title." },
        facebookDescription: {
          label: "Line on Facebook",
          hint: "Leave it empty to use the description.",
        },
        facebookImage: {
          label: "Image on Facebook",
          hint: "Leave it empty to use the feature image.",
        },
        unpublishAt: {
          label: "Take it down at",
          hint: "In your own time zone. At this time the post stops being public. Leave it empty to keep it up.",
        },
      },
      robots: [
        { value: "", label: "Use the blog's setting" },
        { value: "noindex", label: "Keep it out of search results" },
        { value: "noindex, nofollow", label: "Keep it out, and do not follow its links" },
      ],
    },
  },
  goLive: {
    title: "When it goes live",
    hint: "At the time you pick, Khalid publishes the post just as if you had pressed Publish. The checks must still pass.",
    now: "As soon as I press Publish",
    later: "At a time I pick",
    at: "Date and time",
    atHint: "In your own time zone. This is also the date readers see on the post.",
  },
  preview: {
    title: "Show it before it goes live",
    hint: "A preview link lets someone read this draft without signing in. Search engines are told not to list it.",
    make: "Copy preview link",
    label: "Preview link",
    until: (copied: boolean, until: string) =>
      copied
        ? `Copied. Anyone with it can read the draft until ${until}.`
        : `Copy it from this field. Anyone with it can read the draft until ${until}.`,
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
    schedule: "Schedule",
    scheduling: "Scheduling",
    scheduled: "Scheduled to go live",
    blocked: "Fix what stops publishing, then publish again.",
    checks: "Checks",
    noChecks:
      "Khalid checks the post each time it saves, and again when you publish. Anything that stops it, or could make it better, shows here.",
    score: (value: number) => `Score: ${value} out of 100.`,
    scoreWhy: (lost: readonly { points: number }[]) => {
      const points = lost.reduce((sum, rule) => sum + rule.points, 0);
      if (lost.length === 0) {
        return "Nothing found costs points.";
      }
      const what = lost.length === 1 ? "The check below costs" : "The checks below cost";
      return `${what} ${points} points. The score sums up the checks; the checks say what to fix.`;
    },
    blockTag: "Stops publishing",
    warnTag: "Warning, you can still publish",
    here: "Here:",
    leave: "Some changes are not saved yet. Leaving now loses them.",
    stay: "Stay on this post",
    leaveAnyway: "Leave and lose them",
  },
  search: {
    title: "How it looks in Google",
    lede: "A search result for this post, made from the same title and description the published page will carry.",
    result: "Search result preview",
    none: "No description yet. Google picks a line from the post instead.",
    lengths: (title: number, description: number) =>
      `Title ${title} of about 60 characters. Description ${description} of about 160.`,
    seoTitle: "Title in search results",
    seoTitleHint:
      "Leave it empty to use the post's title and the blog's name. {title} and {site} put those in.",
    description: "The line under the title is the Description in Post settings.",
    focus: "Focus keyword",
    focusHint:
      "The search a reader types to find this post. The checks look for it in the title, the description and the first paragraph.",
  },
  schema: {
    title: "Schema",
    lede: "Schema is information for search engines, written in a format called JSON-LD. Khalid writes it for every post. This post's page tells them it holds:",
    types: "Kinds of thing on this page",
    show: "Show the JSON-LD",
    test: "Test the published page in Google's Rich Results Test",
    own: "Your own schema",
    ownHint:
      "For a type Khalid does not write, such as an Event. Paste JSON-LD here; Khalid checks it before it saves. Most posts need nothing here.",
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
    badTime: {
      what: "The time to go live is not complete.",
      why: "Khalid needs a whole date and time to schedule the post.",
      fix: "Fill in the date and the time, or pick As soon as I press Publish.",
    },
    noFile: {
      what: "No CSV file is picked yet.",
      why: "Khalid adds the redirects from a file, so there is nothing to add.",
      fix: "Pick a CSV file under Add redirects from a CSV file, then press Add these redirects.",
    },
    badAlias: {
      what: "The old address is not a path on this blog.",
      why: "Only an address that starts with / and has no spaces can lead to a post.",
      fix: "Write it as it appears after your domain, such as /2019/05/old-title/.",
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
