import type { AdminPost, PostEdit } from "../api/posts";

function pad(value: number): string {
  return String(value).padStart(2, "0");
}

/** A moment as a date and time field holds it, `YYYY-MM-DDTHH:mm`, in the writer's time zone. */
export function localInput(iso: string): string {
  const at = new Date(iso);
  const date = `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`;
  return `${date}T${pad(at.getHours())}:${pad(at.getMinutes())}`;
}

/** The time a post is scheduled for, as the field holds it; empty when it goes live on Publish. */
export function scheduledAt(post: AdminPost): string {
  const at = post.published_at;
  return post.status === "scheduled" && at !== null && at !== undefined ? localInput(at) : "";
}

/**
 * The edit that sends the post live: now when `at` is empty, else at that
 * local time, sent with its offset. `null` when `at` is no time at all.
 */
export function goLiveEdit(updatedAt: string, at: string): PostEdit | null {
  if (at === "") {
    return { updated_at: updatedAt, status: "published" };
  }
  const when = new Date(at);
  if (Number.isNaN(when.getTime())) {
    return null;
  }
  return { updated_at: updatedAt, status: "scheduled", published_at: when.toISOString() };
}
