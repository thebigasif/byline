// The Search Console screen's words, kept beside en.ts so that file stays short.
export const searchConsoleWords = {
  title: "Search Console",
  lede: "What Google Search Console, Google's report of how your blog does in search, says about your posts. Khalid asks Google with your own Google account each time you open this page, so it can take a few seconds.",
  loading: "Asking Google",
  open: {
    title: "Questions your posts do not answer yet",
    count: (open: number, all: number) => `${open} of ${all}`,
    sentence: (first: string, last: string) =>
      `long searches that showed your posts from ${first} to ${last} have no heading that answers them.`,
    allAnswered: (all: number) =>
      all === 1
        ? "The one long search that showed your posts has a heading that answers it."
        : `Each of the ${all} long searches that showed your posts has a heading that answers it.`,
    none: (words: number, first: string, last: string) =>
      `No search of ${words} words or more showed your posts from ${first} to ${last}.`,
    longMeans: (words: number) =>
      `A long search has ${words} words or more. People ask AI assistants in whole questions like these, and each one is a question a reader brings to your post.`,
    fix: "To answer one, add a heading to the post that asks the question, with a 40 to 60 word answer under it. Google and AI assistants can then quote that passage.",
  },
  posts: {
    title: "Each post",
    terms: [
      { term: "Shown", meaning: "How many times Google showed your post for the search." },
      {
        term: "Average position",
        meaning: "Where your post stood in Google's results, 1 being the top.",
      },
    ],
    search: "Search",
    shown: "Shown",
    clicks: "Clicks",
    position: "Average position",
    answeredBy: "Answered under",
    noHeading: "No heading yet",
    heading: (heading: string) => `"${heading}"`,
  },
  window:
    "The numbers cover the last 28 days Google has final numbers for, which end 3 days ago. Google leaves out searches too rare to share, so a small blog may see few or none.",
};
