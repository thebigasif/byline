// The words of the Trash and Audit log screens and of changing many posts
// at once (brief G52). The admin's other words are in en.ts.

/** How a deleted post's old status reads. */
const was = { draft: "Draft", published: "Live", scheduled: "Scheduled" } as const;

/** What each part of the Admin API holds, by the first part of a route after its base. */
const things: Record<string, string> = {
  posts: "a post",
  trash: "a post in the trash",
  redirects: "a redirect",
  "not-found": "an address that found nothing",
  comments: "a comment",
  webhooks: "a webhook",
  media: "a media file",
  staff: "a staff account",
};

/** How each action of the audit log starts its sentence. */
const doing: Record<string, string> = { added: "Added", changed: "Changed", removed: "Removed" };

/** What one change in the audit log did, in words, from its action and route. */
function happened(action: string, route: string): string {
  if (route.endsWith("/restore/")) {
    return "Put a post back from the trash";
  }
  if (route.endsWith("/preview-link/")) {
    return "Made a preview link for a post";
  }
  const part = route.replace("/api/admin/v1/", "").split("/")[0] ?? "";
  const thing = things[part] ?? "a setting";
  return `${doing[action] ?? "Changed"} ${thing}`;
}

export const records = {
  trash: {
    title: "Trash",
    lede: "Posts you deleted wait here until you put them back or delete them for good. Readers never see them.",
    loading: "Reading the trash",
    none: "The trash is empty. A post you delete waits here until you decide what to do with it.",
    count: (n: number) => (n === 1 ? "1 post is in the trash." : `${n} posts are in the trash.`),
    titleColumn: "Title",
    was: "Was",
    status: was,
    deleted: "Deleted",
    action: "What to do",
    restoreText: "Put it back",
    restore: (title: string) => `Put back ${title}`,
    restored: (title: string) => `${title} is back where it was.`,
    purgeText: "Delete for good",
    purge: (title: string) => `Delete ${title} for good`,
    sureText: "Yes, delete it for good",
    sure: (title: string) => `Yes, delete ${title} for good`,
    asking: (title: string) => `Delete ${title} for good? This cannot be undone.`,
    purged: (title: string) => `${title} is deleted for good.`,
    beforeTitle: "Before you delete for good",
    before:
      "Putting a post back returns it to where it was. A live post goes live again at the same address. Deleting for good removes the file, and it cannot be undone.",
  },
  audit: {
    title: "Audit log",
    lede: "Every change made to the blog through the admin, the API or an agent: who made it, what it touched and when. Only owners and admins see this page.",
    who: "Who",
    whoMeans:
      "The key or the staff account that made the change. Each key and account has a short code; the API keys page shows which is which.",
    loading: "Reading the audit log",
    none: "Nothing has been changed through the admin or the API yet.",
    when: "When",
    what: "What happened",
    where: "Where",
    happened,
    older: "Show older changes",
    newest: "Back to the newest",
  },
} as const;
