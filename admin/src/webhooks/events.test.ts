import { describe, expect, it } from "vitest";
import { EVENT_GROUPS, eventSentence, hostOf } from "./events";

describe("webhook events in words", () => {
  it("offers every event in the API's catalogue, each once", () => {
    const offered = EVENT_GROUPS.flatMap((group) => group.events);
    expect(offered).toHaveLength(33);
    expect(new Set(offered).size).toBe(offered.length);
    expect(offered).toContain("changelog-entry.published.edited");
  });

  it("says each event as a sentence", () => {
    expect(eventSentence("post.published")).toBe("A post goes live");
    expect(eventSentence("case-study.published.edited")).toBe("A case study is edited while live");
    expect(eventSentence("comment.added")).toBe("A reader writes a comment");
  });

  it("shows an event it does not know by its API name", () => {
    expect(eventSentence("member.added")).toBe("member.added");
    expect(eventSentence("post.archived")).toBe("post.archived");
    expect(eventSentence(".published")).toBe(".published");
  });

  it("names the host an address points at", () => {
    expect(hostOf("https://hooks.zapier.com/hooks/catch/1/")).toBe("hooks.zapier.com");
    expect(hostOf("not an address")).toBe("not an address");
  });
});
