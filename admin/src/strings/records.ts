// The words of the Trash and Audit log screens and of changing many posts
// at once (brief G52). The admin's other words are in en.ts.

/** How a deleted post's old status reads. */
const was = { draft: "Draft", published: "Live", scheduled: "Scheduled" } as const;

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
} as const;
