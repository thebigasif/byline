# The Search Console screen, 2026-10-31

The founder said yes in advance, on 2026-10-27, to every admin mockup in this phase. This records that yes for the Search Console screen, so the screen can be built without stopping for a review.

## What was agreed

- The mockup is `docs/design/admin-mockups/search-console.html`, beside the other admin mockups.
- The screen is "Search Console" in the side menu, after Redirects, and after Analytics once that lands. It is its own screen, not a section of Analytics: Analytics counts on Khalid's own server, while this one asks Google with the owner's account and can take a few seconds.
- It reads one Admin API route, `GET /api/admin/v1/search-console/follow-ups/`, which needs `settings:write`, as the analytics routes do, because the numbers come from the owner's Google account (GEO M3).
- One focus point: how many of the long searches have no heading that answers them, with what to do about it. There is no button: the period is the 28 days Google has final numbers for, as the `khalid search-console follow-ups` command uses.
- Each post is a table of its searches. An open search says "No heading yet"; an answered one names the heading.
- A site that is not connected sees the API's own what, why and fix instead, calmly, not as an alarm.
- Every term is defined where it first appears: Search Console, long search, shown, average position.

## Left for later

- The rest of Search Console on this screen: each post's numbers, falling posts, missing pages and the week's movers (G43).
- A link from each post to its editor, which needs the post's identity in the route's answer.
