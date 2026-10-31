# Connect to SerpDad, 2026-10-31

The SerpDad draft target (brief G56, KH-API-6) takes finished articles as linted drafts. Plan section 3 says connecting works like WordPress: the blog's address plus a key made in the admin under Settings, SerpDad, and that the page is off until the owner turns it on. This page is that screen. The founder said yes to admin mockups in advance on 2026-10-27 ("you got my all yes. dont stop any work for my yes"), so the mockup `docs/design/admin-mockups/serpdad.html` counts as approved and the screen is built in the same run.

## Decisions

- **Off until a key exists.** "On" means SerpDad has a key that works. Turning it on makes an integration key named `SerpDad` with the one scope `posts:write`, so SerpDad can add drafts and cannot publish, delete or change settings. Turning it off revokes that key. There is no separate switch to forget.
- **One SerpDad key at a time.** Making a new key revokes the old one in the same request, so a lost key is fixed by making a new one, and there is never a forgotten second key.
- **The key is shown once**, the same way `khalid keys create` and the webhook secret are shown. The page shows the blog's address beside it, since SerpDad needs both.
- **Drafts received are recorded as they arrive.** When a draft is made with the SerpDad key, Khalid records its identity, title, blocks and warnings and when it arrived, in a visitor table `serpdad_drafts`, so a rebuild keeps it. The page lists the last 20, newest first, each linking to the editor. A draft deleted later stays in the list as a record; its link then opens the editor's "not found" answer.
- **Recording never fails the draft.** If the record cannot be written, the draft is still made and answered, and the failure is logged. SerpDad's answer is the thing that matters.
- **Who may connect.** The route needs `settings:write`: the owner, an admin, or an integration key given that scope. An editor sees the API's refusal in plain words.
- **One route, `/api/admin/v1/serpdad/`**: `GET` reads the connection and the drafts, `POST` makes the key, `DELETE` turns it off. It does not wait for the general key routes of brief G24, which are not on `main`, and it stays when they land, because it also records and lists the drafts.
- **"SerpDad" in the menu**, the name the owner sees in SerpDad itself. The page title says "Connect to SerpDad", as plan section 3 names it.
