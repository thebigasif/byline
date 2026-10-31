import { passkeys } from "./passkeys";
import { records } from "./records";
import { sitesWords } from "./sites";
import { teamWords } from "./teamWords";
import { webhookWords } from "./webhookWords";

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
    media: "Media",
    comments: "Comments",
    authors: "Authors",
    trash: "Trash",
    audit: "Audit log",
    redirects: "Redirects",
    templates: "Schema templates",
    passkeys: "Passkeys",
    analytics: "Analytics",
    keys: "API keys",
    webhooks: "Webhooks",
    team: "Team",
    settings: "Settings",
  },
  templates: {
    title: "Schema templates",
    lede: "Schema is information for search engines about what a page holds. Khalid writes it for every post. A template adds one more kind to every post in a collection, so every guide is marked as a course without anyone thinking about it.",
    loading: "Reading the schema templates",
    restart:
      "Saved to site.toml. The blog uses it after Khalid restarts; until then, previews show the templates it started with.",
    inUse: "Templates in use",
    none: "No templates yet. Every post still carries the schema Khalid writes for it.",
    every: (target: string, name: string, type: string) =>
      target === "category"
        ? `Every post in the category ${name} is a ${type}`
        : `Every post in ${name} is a ${type}`,
    remove: "Remove this template",
    form: {
      title: "Add a template",
      collection: "Collection",
      collectionHint: "Every post in it gets the schema.",
      type: "Kind of thing, as schema.org names it",
      typeHint:
        "One word starting with a capital, such as Course, Recipe or Event. Google lists the kinds it shows in results.",
      details: "Details",
      detailsHint:
        "Each detail has a name schema.org uses and a value. A value may take words from the post: {title}, {excerpt}, {author}, {site}.",
      name: "Name",
      value: "Value",
      more: "Add another detail",
      save: "Save templates",
      saving: "Saving the templates",
    },
  },
  trash: records.trash,
  audit: records.audit,
  bulk: records.bulk,
  passkeys,
  media: {
    title: "Media",
    lede: "The images, videos, sound and files your posts use. Khalid reads each file's content to learn its type, so a program renamed to look like a photo is refused. A file uploaded twice is kept once.",
    add: {
      title: "Add a file",
      file: "File",
      fileHint:
        "A JPEG, PNG or WebP image up to 20 MB, a video up to 100 MB, sound up to 50 MB, or a PDF, ZIP or text file.",
      folder: "Folder",
      folderHint:
        "Optional. Lowercase words joined by /, such as guides/2026. Leave it empty for the top of the library.",
      submit: "Upload",
      uploading: "Uploading the file",
      added: (name: string) => `Added. ${name} is in your library.`,
      addAlt: "Give it alt text next, so readers who cannot see it know what it shows.",
      duplicate: (name: string) => `${name} was already in your library, so nothing new was added.`,
    },
    library: {
      title: "Your library",
      lede: "Newest first. Alt text describes an image for people who use a screen reader, and for Google. The checks warn about any image in a post that has none.",
      show: "Show",
      every: "Every file",
      noAlt: "Images with no alt text",
      loading: "Reading your library",
      empty: "Your library is empty. Add a file above to start it.",
      allHaveAlt: "Every image in your library has alt text.",
      found: (count: number) => (count === 1 ? "1 file" : `${count} files`),
      preview: "Preview",
      name: "Name",
      folder: "Folder",
      kind: "Kind",
      alt: "Alt text",
      top: "Top",
      missing: "Missing",
      notNeeded: "Not needed",
      action: "Action",
      edit: (name: string) => `Edit details of ${name}`,
      editText: "Edit details",
    },
    details: {
      back: "Back to media",
      loading: "Reading the file's details",
      uploaded: (summary: string, by: string, on: string) =>
        `${summary}. ${by} uploaded it on ${on}.`,
      address: "Its address:",
      copy: "Copy the address",
      copied: "Copied. Paste it into a post to use the file.",
      title: "Details",
      alt: "Alt text",
      altHint:
        "What the image shows, for people who use a screen reader and for Google. One sentence is enough.",
      caption: "Caption",
      captionHint: "Optional. The line shown under it in a post.",
      folderHint:
        "Lowercase words joined by /, such as guides/2026. Empty means the top of the library. Its address stays the same.",
      point: "Focal point",
      pointHint:
        "The ring marks the focal point, the part every crop keeps. Click the picture to move it, or type the two numbers.",
      picture: "Move the focal point to where you click",
      left: "From the left, in %",
      top: "From the top, in %",
      save: "Save details",
      saving: "Saving",
      saved: "Saved.",
    },
    kinds: { image: "Image", video: "Video", audio: "Sound", file: "File" },
    size: (bytes: number) =>
      bytes < 1024 * 1024
        ? `${Math.max(1, Math.round(bytes / 1024))} KB`
        : `${(bytes / (1024 * 1024)).toFixed(1)} MB`,
    pixels: (width: number, height: number) => `${width} × ${height}`,
  },
  analytics: {
    title: "Analytics",
    lede: "How many people read your blog, where they came from, and what AI assistants do with it. Khalid counts on its own server, with no cookies, so readers who block scripts count too. It never keeps who a reader is.",
    loading: "Counting the page views",
    total: (days: number) => `Page views in the last ${days} days`,
    rise: (percent: number, days: number, before: string) =>
      `${percent}% more than the ${days} days before, which had ${before}.`,
    fall: (percent: number, days: number, before: string) =>
      `${percent}% fewer than the ${days} days before, which had ${before}.`,
    level: (days: number) => `The same as the ${days} days before.`,
    fresh: (days: number) => `The ${days} days before had none to compare with.`,
    viewMeans:
      "A page view is one page opened by a person. A reader who opens two pages counts twice. Bots never count.",
    days: "Days to count",
    period: (days: number) => `The last ${days} days`,
    show: "Show these days",
    chart: {
      title: "Each day",
      none: "No page views were counted in these days.",
      summary: (least: string, most: string, busiest: string) =>
        `From ${least} to ${most} page views a day. The busiest day was ${busiest}, with ${most}.`,
      table: "Show each day as a table",
      day: "Day",
    },
    views: "Page views",
    before: (days: number) => `The ${days} days before`,
    sources: {
      title: "Where readers came from",
      lede: (arrivals: string) =>
        `${arrivals} of the page views came from outside the blog. The rest were readers moving from one of your pages to another.`,
      from: "From",
      names: {
        direct: "Typed in or bookmarked",
        search: "Search engines, such as Google",
        social: "Social sites",
        ai: "AI assistants, such as ChatGPT",
        other: "Other websites",
        internal: "Your own pages",
      },
    },
    pages: {
      title: "Most read pages",
      page: "Page",
      none: "No page was read in these days.",
    },
    tools: {
      title: "Readers sent by AI assistants",
      lede: "An AI assistant, such as ChatGPT or Perplexity, sent these readers when it named your page in an answer and the reader clicked through.",
      tool: "Assistant",
      top: "Page they opened most",
      noPage: "None in these days",
      none: "No AI assistant sent a reader in these days or the days before.",
    },
    uncited: {
      title: "Read by AI, but never cited",
      lede: "AI bots, the programs AI companies send to read the web, read these pages. Yet no AI assistant sent a single reader to them. The assistants know the page but do not point people to it. To be cited, start each section with a short, direct answer to the question its heading asks.",
      reads: "Reads by AI bots",
      bots: "Bots",
      last: "Last read",
      none: "No page fits here: every page AI bots read in these days also had a reader sent by an AI assistant, or AI bots read none.",
      after:
        "A bot is known by the name it sends, which anyone can copy. Khalid keeps what bots read for 90 days.",
    },
  },
  authors: {
    title: "Authors",
    lede: "An author is a person named on a post. Their name, role and profiles show beside every post they write, and tell search engines who stands behind it. Each author is a file in your site folder, so git keeps their history.",
    add: {
      title: "Add an author",
      fields: {
        name: { label: "Name", hint: "As readers see it under a post's title." },
        slug: {
          label: "Short name",
          hint: "Posts name the author by it. Lowercase words joined by hyphens, such as lena-vogt.",
        },
        role: { label: "Role", hint: "What they do, such as Technical SEO editor. Optional." },
        language: {
          label: "Language",
          hint: "The author page's language, such as de. Leave it empty for the blog's main one.",
        },
      },
      submit: "Add author",
      added: (name: string) => `Added. Posts can now name ${name} as their author.`,
    },
    list: {
      title: "Your authors",
      loading: "Reading your authors",
      none: "No authors yet. Add the first one above.",
      name: "Name",
      slug: "Short name",
      language: "Language",
      role: "Role",
      noRole: "No role yet",
      action: "Action",
      edit: "Edit",
      editLabel: (name: string, language: string) => `Edit ${name}, ${language}`,
    },
    edit: {
      title: "Author",
      back: "All authors",
      loading: "Reading the author",
      lede: (slug: string, language: string) =>
        `Posts name this author ${slug}. This is their ${language} page; each language has its own.`,
      fields: {
        name: { label: "Name", hint: "As readers see it under a post's title." },
        role: { label: "Role", hint: "What they do. Leave it empty for none." },
        credentials: {
          label: "Why to trust them",
          hint: "Such as ten years running SEO for small teams. Search engines read it.",
        },
        photo: { label: "Photo", hint: "A media path or a full address. Leave it empty for none." },
        bio: { label: "Bio", hint: "A few sentences about them, in markdown." },
        sameAs: {
          label: "Profiles elsewhere",
          hint: "Full addresses, one per line, such as their LinkedIn page. Search engines use them to tell people apart.",
        },
      },
      save: "Save author",
      saved: "Saved. Every post that names this author shows the change.",
      unchanged: "Nothing changed, so nothing was saved.",
      remove: {
        title: "Delete this author",
        lede: (name: string) =>
          `Deleting removes ${name}'s file. A post that still names them loses its byline, so give those posts another author first.`,
        submit: (name: string) => `Delete ${name}`,
      },
    },
  },
  keys: {
    title: "API keys",
    lede: "An API key lets another program use your blog: your website's code, SerpDad, or a script a teammate runs. Each key can do only what you allow, and you can switch it off at any time.",
    kinds: { content: "Content key", integration: "Integration key", staff: "Staff token" },
    scopes: {
      "content:read": "Read live posts",
      "posts:write": "Write drafts",
      "posts:publish": "Publish posts",
      "media:write": "Upload images and files",
      "settings:write": "Change settings",
      "webhooks:manage": "Manage webhooks",
      "subscribers:write": "Change subscribers",
      "admin:all": "Everything",
    },
    asRole: (role: string) => `Everything a ${role} may do`,
    noRole: "staff member",
    create: {
      title: "Create a key",
      name: "Name",
      nameHint: "So you know later which program uses it, such as Main site.",
      kind: "What kind of key?",
      kindHints: {
        content: "Reads posts that are already live, and nothing else. Safe to put in a web page.",
        integration:
          "Lets a tool such as SerpDad write posts, upload images or change settings. Keep it secret.",
      },
      scopes: "What may it do?",
      once: "You see a new key only once. Khalid keeps a scrambled copy that nobody can turn back into the key, not even you. Copy it into the other program straight away.",
      submit: "Create key",
    },
    shown: {
      title: (name: string) => `Your new key for ${name}. Copy it now: it is not shown again.`,
      label: "The key",
      hint: "Select it and copy it into the program that will use it.",
    },
    list: {
      title: "Keys in use",
      lede: "Switching a key off works at once. The program using it is refused from its next request, so make sure it has a new key first.",
      loading: "Reading your keys",
      name: "Name",
      kind: "Kind",
      may: "May do",
      made: "Made",
      action: "Action",
      revoke: "Switch off",
      revokeLabel: (name: string) => `Switch off the key ${name}`,
      revoked: (name: string) => `Switched off. The key ${name} no longer works.`,
    },
  },
  webhooks: webhookWords,
  team: teamWords,
  site: {
    title: "Settings",
    lede: "These settings cover the whole blog. They are saved to site.toml in your site folder, the same file you can edit by hand.",
    loading: "Reading your settings",
    where: "Name and address",
    languages: "Languages",
    fields: {
      name: {
        label: "Blog name",
        hint: "Shown in the browser tab, in feeds and in search results.",
      },
      address: {
        label: "Main site address",
        hint: "Your website's address, such as https://www.example.com.",
      },
      mount: { label: "Folder", hint: "Where the blog lives on that site, such as /blog/." },
      languages: {
        label: "Languages",
        hint: "Language codes separated by commas, such as en, de. Each post can have a version in each.",
      },
      default_language: {
        label: "Main language",
        hint: "The language visitors see first.",
      },
    },
    careful:
      "Changing the address or the folder changes the link to every post, feed and sitemap. Add a redirect for the old addresses, or links from other sites will break.",
    restart:
      "Restart Khalid on the server to use these settings. The running blog read site.toml when it started, and the file has changed since.",
    save: "Save settings",
    saved: "Saved to site.toml.",
    unchanged: "Nothing changed, so nothing was saved.",
  },
  account: {
    nav: "Your account",
    title: "Your account",
    loading: "Reading your account",
    lede: (name: string, email: string) =>
      `Signed in as ${name}, ${email}. Here you choose how you sign in, and lock everyone else out if a laptop goes missing.`,
    passkey: {
      title: "Passkeys",
      lede: "A passkey lets you sign in with your fingerprint, your face or your device's PIN instead of a password. It cannot be guessed or stolen by a fake sign-in page.",
      link: "Add, rename or remove your passkeys",
    },
    password: {
      title: "Change your password",
      lede: "Changing it signs you out on every other device.",
      current: "Current password",
      currentHint:
        "Leave it empty only when you have no password yet. A first one can be set in the 15 minutes after you sign in.",
      next: "New password",
      nextHint: "At least 12 characters. A short sentence is easy to remember.",
      submit: "Change password",
      changed: "Changed. Every other device is signed out.",
    },
    everywhere: {
      title: "Sign out everywhere",
      lede: "For a lost laptop or a phone someone else picked up. Every device signed in as you is signed out, this one too, and you sign in again.",
      submit: "Sign out everywhere",
    },
  },
  comments: {
    title: "Comments",
    lede: "Readers' comments wait here until you approve them, so nothing appears under a post without your yes. Approve the good ones, answer questions, and delete spam.",
    which: "Which comments",
    statuses: { pending: "Waiting", approved: "Shown", spam: "Spam", trash: "Bin" },
    loading: "Reading the comments",
    none: {
      pending: "No comment is waiting for you.",
      approved: "No comment shows on the blog yet.",
      spam: "No comment is marked as spam.",
      trash: "The bin is empty.",
    },
    on: (post: string, when: string) => `on ${post}, ${when}`,
    staff: "Staff",
    approve: "Approve",
    approveLabel: (name: string) => `Approve the comment from ${name}`,
    remove: "Delete",
    removeLabel: (name: string) => `Delete the comment from ${name}`,
    reply: "Reply as staff",
    replyHint: "Your reply shows at once, and approves the comment it answers.",
    name: "Your name",
    nameHint: "Shown with your reply, such as your first name.",
    text: "Your reply",
    send: "Send reply",
    approved: (name: string) => `Approved. The comment from ${name} now shows under its post.`,
    removed: (name: string) => `Deleted the comment from ${name} and every reply to it.`,
    replied: (name: string) => `Sent. Your reply to ${name} shows under the post.`,
    after:
      "Delete removes a comment and every reply to it, for good. A reader's email address is never shown on the blog.",
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
  // The name each block card shows in the rich-text view.
  blocks: {
    audio: "Audio",
    bookmark: "Bookmark card",
    business: "Business card",
    button: "Button",
    callout: "Callout",
    embed: "Embed",
    faq: "FAQ",
    figure: "Image with caption",
    file: "File download",
    form: "Form",
    gallery: "Gallery",
    gated: "Subscribers only",
    header: "Header",
    howto: "How-to",
    html: "HTML",
    maths: "Maths",
    newsletter: "Newsletter sign-up",
    product: "Product card",
    recipe: "Recipe",
    review: "Review",
    toc: "Table of contents",
    toggle: "Toggle",
    video: "Video",
    table: "Table",
    footnote: "Footnote",
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
    email: {
      title: "Or sign in with your email",
      lede: "For an account the owner made for you. It lets you add a passkey under Your account.",
      address: "Email",
      password: "Password",
      submit: "Sign in with email",
      checking: "Checking your email and password",
    },
    link: {
      title: "Signing you in",
      checking: "Checking your sign-in link",
      back: "Go to the sign-in page",
    },
  },
  sites: sitesWords,
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
    noMediaItem: {
      what: "This file is not in the media library.",
      why: "It may never have been uploaded, or the address is mistyped.",
      fix: "Go back to Media and open the file from the list.",
    },
    noMedia: {
      what: "No file is picked yet.",
      why: "Khalid adds the file you pick, so there is nothing to upload.",
      fix: "Pick a file under File, then press Upload.",
    },
    notYet: {
      what: "This blog's server cannot do this yet.",
      why: "It runs a version of Khalid from before this screen was added.",
      fix: "Update Khalid on the server with khalid update, then reload this page.",
    },
    noSession: {
      what: "Your account settings need you signed in with your email.",
      why: "This tab signed in with a staff token, and a token belongs to no account.",
      fix: "Sign out, then sign in with your email or with a sign-in link.",
    },
    badLink: {
      what: "This sign-in link no longer works.",
      why: "Each link works once, for a short time, so nobody else can use it later.",
      fix: "Ask the blog's owner for a new link, or sign in with your email and password.",
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
