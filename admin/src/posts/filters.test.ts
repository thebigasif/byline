import { describe, expect, it } from "vitest";
import { PAGE_SIZE, postSearch, rowQuery } from "./filters";

describe("the posts screen's address", () => {
  it("keeps the filters it knows and drops the rest", () => {
    const search = postSearch({ checks: "blocked", stage: "nope", index: "noindex", page: "3" });
    expect(search).toEqual({ checks: "blocked", stage: undefined, index: "noindex", page: 3 });
  });

  it("reads page 1 and nonsense pages as no page", () => {
    expect(postSearch({ page: "1" }).page).toBeUndefined();
    expect(postSearch({ page: "-2" }).page).toBeUndefined();
    expect(postSearch({ page: "2.5" }).page).toBeUndefined();
  });

  it("asks for newest first with no filter when nothing is picked", () => {
    expect(rowQuery({})).toEqual({
      filter: undefined,
      order: "updated_at desc",
      limit: PAGE_SIZE,
      page: 1,
    });
  });

  it("joins every picked filter with +", () => {
    const query = rowQuery({ checks: "blocked", stage: "in_review", index: "noindex", page: 2 });
    const all = "gate:refuses+status:draft+review_state:in_review+index_status:noindex";
    expect(query.filter).toBe(all);
    expect(query.page).toBe(2);
    const live = rowQuery({ checks: "clear", stage: "live" });
    expect(live.filter).toBe("gate:passes+status:published");
  });
});
