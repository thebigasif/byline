# khalid-auth

Staff accounts for the admin: who may sign in, and what each person may do. A staff member signs in with an email and password, a passkey or a link sent by email. A passkey is a key kept on the person's phone or computer, unlocked by a fingerprint, a face or a PIN.

## How staff get in

1. The owner runs `khalid staff owner --email you@example.com --name "Your name"` on the server. It makes the owner when the site has none and shows a sign-in link once. The link works for 15 minutes, one time.
2. Signed in, the owner chooses a password or adds a passkey, and invites the rest of the staff.
3. Invited people sign in by a link sent by email, once the site can send mail, and then do the same.

Staff tokens made with `khalid keys create --kind staff` keep working beside all of this, for tools and agents.

Every account has one role. The roles are the same ones a staff token carries (ADR 0011), so a token and a signed-in person with the same role may do the same things.

## What each role may do

A test reads this table and checks it is the one the code grants.

| Permission | owner | admin | editor | writer | contributor |
|---|---|---|---|---|---|
| `posts:write-own` | yes | yes | yes | yes | yes |
| `posts:publish-own` | yes | yes | yes | yes | no |
| `posts:edit-others` | yes | yes | yes | no | no |
| `media:upload` | yes | yes | yes | yes | no |
| `taxonomy:manage` | yes | yes | yes | no | no |
| `comments:moderate` | yes | yes | yes | no | no |
| `subscribers:manage` | yes | yes | no | no | no |
| `settings:manage` | yes | yes | no | no | no |
| `integrations:manage` | yes | yes | no | no | no |
| `staff:manage` | yes | yes | yes | no | no |
| `site:transfer` | yes | no | no | no | no |

- `posts:write-own`: write drafts and edit the posts they wrote. A contributor's draft waits for an editor to publish it.
- `posts:edit-others`: edit, publish and delete anyone's posts.
- `taxonomy:manage`: tags, categories, authors, collections and snippets.
- `settings:manage`: site settings, the design, navigation and redirects.
- `integrations:manage`: API keys and webhooks.
- `site:transfer`: hand the site to another account. A site has exactly one owner.

## Which accounts each role may manage

To manage an account is to invite it, change its role or remove it. A role change needs the right to manage both the old role and the new one, so nobody hands out more than they hold. Each row is the role doing the managing, and each column is the role of the account it manages.

| Manages | owner | admin | editor | writer | contributor |
|---|---|---|---|---|---|
| `owner` | no | yes | yes | yes | yes |
| `admin` | no | yes | yes | yes | yes |
| `editor` | no | no | no | yes | yes |
| `writer` | no | no | no | no | no |
| `contributor` | no | no | no | no | no |

Nobody manages the owner, not even the owner: the site changes hands only by a transfer.
