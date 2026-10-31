# Running Khalid on a server

Khalid is one file. Put it on a machine, point it at a site folder, and it serves the blog and the admin. This page shows the ways to keep it running. The files it names are in `packaging/` in the repository.

A site folder is the folder that holds `site.toml`, `content/` and `theme/`. Khalid keeps what it works out from those files, such as its database and image sizes, in `data/` inside the same folder. Back up the site folder and you have backed up the blog.

## Install with one line

On Linux or a Mac:

```sh
curl -fsSL https://raw.githubusercontent.com/thebigasif/khalid/main/scripts/install.sh | sh
```

The script downloads the newest release for your machine and checks it against the release's list of checksums, `SHA256SUMS`. A checksum is a fingerprint of a file: if the download was damaged or changed, the fingerprints differ and the script stops without installing anything. As root it installs to `/usr/local/bin`, otherwise to `~/.local/bin`.

Options, written after `sh -s --`:

- `--version v0.1.0` installs that release instead of the newest.
- `--dir FOLDER` installs into another folder.
- `--systemd`, as root, also sets up the service described below.

For example, to install and set up the service on a server:

```sh
curl -fsSL https://raw.githubusercontent.com/thebigasif/khalid/main/scripts/install.sh | sudo sh -s -- --systemd
```

## Signing in to the admin the first time

The admin is at `/admin/` on the site's host. The first person in is the owner, and the command line makes them:

```sh
khalid --site blog staff owner --email you@example.com --name "Your Name"
```

This is the only time Khalid shows the sign-in link. The link works once, within 15 minutes. It is shown on two addresses: the site's own, and `localhost`, for a site that is not online yet. Open the `localhost` one while `khalid serve` runs on the same computer. If `khalid serve` listens somewhere other than `127.0.0.1:8080`, pass the same `--listen` to `staff owner`. Once signed in, choose a password or add a passkey.

## systemd on a Linux machine

systemd is the program most Linux systems use to start services at boot. Khalid ships a unit file for it, `packaging/systemd/khalid.service`, and a user file, `packaging/systemd/khalid.sysusers`.

The unit runs Khalid as its own `khalid` user, never as root. The site folder is `/var/lib/khalid`, and the service can write nowhere else. It answers on `127.0.0.1:8080`, so only a program on the same machine, such as a proxy, can reach it.

1. Put the binary at `/usr/local/bin/khalid`.
2. Copy the two files and make the user:

   ```sh
   sudo cp packaging/systemd/khalid.service /etc/systemd/system/
   sudo cp packaging/systemd/khalid.sysusers /etc/sysusers.d/khalid.conf
   sudo systemd-sysusers
   ```

3. Make the site, as the `khalid` user:

   ```sh
   sudo install -d -o khalid -g khalid -m 0750 /var/lib/khalid
   sudo -u khalid khalid init /var/lib/khalid --address https://blog.example.com --yes
   ```

4. Start it now and at every boot:

   ```sh
   sudo systemctl enable --now khalid
   ```

`systemctl status khalid` says whether it runs, and `journalctl -u khalid` shows what it printed.

To change the command, for example to answer on every address of the machine, run `sudo systemctl edit khalid` and write:

```ini
[Service]
ExecStart=
ExecStart=/usr/local/bin/khalid --site /var/lib/khalid serve --listen 0.0.0.0:8080
```

The empty `ExecStart=` line clears the old command first. The unit already lets Khalid answer on ports 80 and 443 without root, for when it gets its own certificate with `serve --domain`.

The unit also sets `Environment=MIMALLOC_PURGE_DELAY=0`, the allocator setting `docs/standards/performance.md` records: it makes mimalloc return freed pages to the OS at once, which keeps the server's idle memory inside its budget. If you run the release binary by hand instead of through the unit, the image or `compose.yaml`, set the same variable in the shell first:

```sh
MIMALLOC_PURGE_DELAY=0 khalid --site /var/lib/khalid serve --listen 127.0.0.1:8080
```

## Docker

Docker runs programs in containers, each with its own files. The `Dockerfile` at the top of the repository builds an image from scratch, which means the image starts empty. It holds the Khalid binary, built fully static so it needs no other library, and the certificates Khalid needs to make HTTPS requests. There is no shell and no package manager in it, so there is nothing else to patch.

The image keeps the site folder at `/site` and answers on port 8080. It runs as user 65532, not as root.

```sh
docker build -t khalid .
mkdir site
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD/site:/site" khalid init /site --address https://blog.example.com --yes
docker run -d -u "$(id -u):$(id -g)" -p 8080:8080 -v "$PWD/site:/site" khalid
```

`-u` runs Khalid as you, so the files it writes in `site/` stay yours. The blog is then at `http://localhost:8080/`.

Each release also publishes the image, for x86_64 and ARM, so you can skip the build. Use `ghcr.io/thebigasif/khalid:latest` in place of `khalid` in the commands above, or a version such as `ghcr.io/thebigasif/khalid:0.1.0` to stay on one release.

## Docker Compose

Docker Compose starts containers from a file, `compose.yaml`, so the settings live in one place. The file in the repository runs the image with the site in `./site`, port 8080, a read-only container and no extra rights.

```sh
mkdir site
docker compose run --rm khalid init /site --address https://blog.example.com --yes
docker compose up -d
```

Compose runs Khalid as user 1000, the first user on most Linux machines. If `id -u` prints another number, put `KHALID_UID=<that number>` and `KHALID_GID=<the number id -g prints>` in a file named `.env` next to `compose.yaml`. When the image cannot be downloaded yet, Compose builds it from the repository.

`KHALID_ADDRESS` changes the site's address without editing `site.toml`, for example when a site moves to a new domain. Uncomment the `environment` lines in `compose.yaml` to set it.

## Hosting platforms

The repository holds a ready file for four hosts. Each one builds the `Dockerfile` or installs the binary, keeps the site on a disk that survives restarts, and makes the site on the first start. `KHALID_ADDRESS` says the blog's address. The image's command, `serve --init`, makes a site there when the disk is empty, and leaves a site it finds as it is.

### Fly.io

Fly.io runs containers close to readers. You need its command line tool, `fly`, and an account.

1. In a copy of this repository, run `fly launch --no-deploy`. It reads `fly.toml` and asks for the app's name.
2. In `fly.toml`, set `KHALID_ADDRESS` to `https://<the app's name>.fly.dev`, or to your own domain once it points at Fly.
3. Run `fly deploy`. Fly builds the image, makes a 1 GB disk for the site, and starts the blog.

The site lives on one disk, so the app runs on one machine. Fly stops it when no one reads and starts it on the next visit.

### Railway

Railway builds the `Dockerfile` with the settings in `railway.json`: one copy of the service, checked at `/` before it takes readers.

1. In Railway, make a project from this repository.
2. Under the service's Settings, Networking, make a public domain, and point it at port 8080.
3. Add a volume mounted at `/site`.
4. Under Variables, add `KHALID_ADDRESS` as `https://${{RAILWAY_PUBLIC_DOMAIN}}`, and `RAILWAY_RUN_UID` as `0`. Railway's volumes belong to the root user, so Khalid must run as root to write to it. Railway still keeps the container apart from the machine.
5. Deploy.

### Coolify

Coolify is a hosting panel you run on your own server.

1. Add a new resource from this repository, with the Docker Compose build pack.
2. Set the Docker Compose location to `/packaging/coolify/compose.yaml`.
3. Deploy.

Coolify makes the blog's address, sends it to port 8080, and passes it to Khalid as `KHALID_ADDRESS`. The site lives in the volume `khalid-site`. To use your own domain, change the domain in the service's settings and deploy again.

### A Hetzner Cloud server

`packaging/hetzner/cloud-config.yaml` sets up a new server in its first minute: it runs the install script with `--systemd`, makes the site, and starts Khalid with HTTPS from Let's Encrypt, the free service that issues certificates. Pasting it means you agree to Let's Encrypt's subscriber agreement.

1. In the Hetzner Cloud console, make a Primary IPv4 address under Networking, and point your blog's domain at it.
2. Copy the file and change `blog.example.com` in it to that domain.
3. Create a server with that address, and paste the file under "Cloud config".

The blog is at `https://<your domain>/` a few minutes later. Point the domain first: Khalid asks for its certificate on the first start, and Let's Encrypt only gives one for a domain that leads to the server.

## Sending email

Khalid sends email when a reader signs up for the newsletter: one message with a link that confirms the address. It also sends you each answer to a form block. Until `site.toml` says how mail is sent, the sign-up form says sign-ups are not open yet and stores nothing.

Gated content, the part of a post inside `:::gated`, opens with the same email. A reader types their address in the form where the part would be, opens the link, clicks the button, and lands back on the post with the part open, with scripts off. Their browser keeps a cookie for 180 days that opens gated content on every post. On another phone or browser they type the same address again and get a link that opens it there. Without `[mail]` the part stays locked for everyone.

Add a `[mail]` table with the address mail comes from and exactly one way to send it.

Through the mail server your host gives you, with SMTP (the protocol mail servers speak):

```toml
[mail]
from = "news@example.com"
name = "Field Notes"
postal_address = "Field Notes Ltd, 1 Park Row, Leeds LS1 5AB, UK"

[mail.smtp]
host = "smtp.example.com"
username = "news@example.com"
```

`postal_address` is the address where people can reach you or your business by post. Anti-spam laws ask every newsletter email to show one, so Khalid sends no newsletter until it is set; sign-up emails work without it.

Khalid connects on port 587 and turns encryption on before it sends anything. Add `port = 465` and `security = "tls"` for a server that is encrypted from the start. A relay on the same machine or your own network needs `private_host = true`; only then may it use `security = "none"`.

Form answers are emailed to the `from` address unless you add `forms_to = "hello@example.com"` under `[mail]`.

Through Resend or Postmark, two services that send mail for websites, write an empty `[mail.resend]` or `[mail.postmark]` instead of `[mail.smtp]`. Postmark sends on its `outbound` stream unless you add `message_stream = "<name>"`.

The password, API key or token never goes in `site.toml`. Set it in the server's environment, where only Khalid reads it:

| Way of sending | Variable |
|---|---|
| SMTP with a `username` | `KHALID_SMTP_PASSWORD` |
| Resend | `KHALID_RESEND_API_KEY` |
| Postmark | `KHALID_POSTMARK_TOKEN` |

With systemd, run `sudo systemctl edit khalid` and add `Environment=KHALID_SMTP_PASSWORD=...` under `[Service]`, or keep it in a file only root can read and name it with `EnvironmentFile=`. With Compose, put it in the `.env` file and pass it on under `environment:` in `compose.yaml`, as `KHALID_SMTP_PASSWORD: ${KHALID_SMTP_PASSWORD}`. Khalid refuses to start, and says which variable is missing, when the way you picked needs one that is not set.

## Alerts

`khalid serve` tells you when something on the blog needs you. Once an hour it checks for a burst of visits ending on missing pages (404s), an HTTPS certificate with under two weeks left, and live posts that have not changed in a year. It also tells you when a scheduled post did not go live because the linter refused it, and `khalid audit` tells you about broken links. Each problem is sent once while it lasts, and again if it clears and comes back.

Every alert goes to the webhooks registered for `alert.raised`, so Slack, Zapier or Make can pass it on:

```sh
khalid webhooks add --event alert.raised --url https://hooks.example.com/khalid
```

To get alerts by email too, set up `[mail]` as above and add the address:

```toml
[alerts]
email = "owner@example.com"
not_found_spike = 50     # new 404s in one hour that count as a burst
stale_after_days = 365   # 0 switches the stale-post alert off
watch_minutes = 5        # how often the speed and uptime watch looks; 0 switches it off
slow_ms = 1000           # a slower middle answer time over an hour is an alert; 0 switches it off
```

### Speed and uptime

Once the server answers more than its own computer, with `--domain` or a `--listen` address such as `0.0.0.0:8080`, it looks at the blog's home page at its public address every 5 minutes, the way a reader's browser would: through DNS, the certificate and any proxy in front. Three failed looks in a row send a "site down" alert. An hour whose middle answer time is over 1 second sends a "slow site" alert. Both are sent once, and again only after the site has recovered.

To see the record, run:

```sh
khalid --site /srv/blog watch
```

It shows, for the last day and the last 30 days, how many looks answered, how fast, and the longest gap between looks. A long gap means the server itself was off, since a stopped server cannot look. To be told about that too, have a second machine or an outside monitor watch the address.

## The visitor log

The visitor log keeps one row per visit for 7 to 14 days, so Khalid can check that a bot is who it says it is and see when a new post was first crawled. It sets no cookie and keeps only shortened addresses, but it is still personal data, so it is off until you switch it on. Ask your lawyer to read `docs/design/visitor-log.md` first.

```toml
[visitor_log]
on = true
keep_days = 14   # 7 to 14
```

While it is on, three commands read it. A crawler counts only when its address is on the list its company publishes, so a script that borrows Googlebot's name is never counted as Googlebot.

- `khalid crawl first`: for each post that went live after the log saw its first crawl, how long each crawler took to fetch it.
- `khalid crawl health`: each search and AI crawler's visits over the kept days, by what the site answered: the page, moved, missing or failed.
- `khalid crawl errors`: each address that answered crawlers with a 4xx or 5xx, most hits first.

Add `--json` to any of them for the same as one JSON object. The hourly alerts also raise `crawl-stopped` when Googlebot or Bingbot has not fetched a page for a week, and `crawler-errors` when crawlers got a server error in the last hour. Both go out like every other alert.

## Many sites on one server

One Khalid process can serve several blogs, each on its own domain. In an empty folder, add each site by its domain. The first one is the home site, where super admins add more:

```sh
khalid sites add example.com
khalid sites add news.example.com
khalid sites admin you@example.com
khalid --site sites/example.com staff owner --email you@example.com --name "Your Name"
khalid serve --listen 127.0.0.1:8080
```

Each site gets its own folder in `sites/`, with its own posts, theme, staff and database, and `khalid --site sites/<domain>` runs any other command on it. `khalid sites list` shows them all.

A super admin is a person whose email `sites.toml` lists and who signs in on the home site. They can add a site from there while the server runs, and it answers at once. Keys and staff of the other sites cannot. The page for it is `/admin/sites` on the home site: it asks for your email, sends you a sign-in link, and opening the link brings you back to it. The sign-in link `khalid staff owner` prints opens the same page.

Point each domain's DNS at the server. HTTPS for many domains is not built in yet, so put a proxy such as Caddy in front: it gets a certificate for each domain and passes requests to `127.0.0.1:8080`.

## Taking your data out

`khalid db export` writes every table of the site's database as a CSV file and a JSON file (two plain formats spreadsheets and reporting tools read), one pair per table, into `data/export/`. Add `--out <folder>` to write them somewhere else. Google Sheets, Excel and Looker Studio open the CSV files as they are.

Sign-in material never leaves: sessions, sign-in links, passkeys and the site's own keys are left out, and so are the hashes of passwords and keys and the secrets that sign webhooks. A text field that a spreadsheet would run as a formula starts with `'`, so opening a file runs nothing.

## Your own code and the consent banner

Some tools ask you to paste code on every page: a tag manager, an analytics snippet or a chat window. Put it in a `[code]` table in `site.toml`.

```toml
[code]
head = """
<script async src="https://www.googletagmanager.com/gtag/js?id=G-1"></script>
<script>gtag('config', 'G-1');</script>"""
body = "<script src='https://chat.example.com/widget.js' defer></script>"
```

`head` goes at the end of every page's head and may hold `script`, `style`, `meta` and `link`. `body` goes at the end of every page's body and may hold only `script`. Khalid checks both when it starts and says what to change if something does not belong.

By default this code waits for each reader's consent, which means their yes. Every page then ends with a short note and two buttons, Allow and Do not allow. The buttons are a plain form, so they work with scripts turned off. The code runs only for a reader who pressed Allow, and a post's own head code waits too. A reader's choice is kept in a cookie for 180 days, and a small form at the bottom of each page lets them change it.

For code that sets no cookie and tracks no one, add `consent = "not-needed"` to `[code]`. The code then runs for every reader and no note is shown.

Each script gets the page's nonce, a random value the browser checks against the page's security policy, so the code is allowed to run and nothing else is. A script may load other scripts and send data to any `https` address, as analytics tools do.

A static site cannot keep a reader's choice, so Khalid refuses to start in `mode = "static"` with code that waits for consent. Set `consent = "not-needed"` or switch to `mode = "server"`. Site verification tags belong in `[seo]`, which never waits.

## Bing Webmaster Tools

Bing Webmaster Tools is Bing's dashboard for site owners. Add the site there with the address in `site.toml`, put the code Bing gives you as `bing` under `[seo.verification]` in `site.toml`, and verify. Then make an API key, the password a program uses to reach Bing for you, under Settings, API Access, and set it as `KHALID_BING_API_KEY` in the environment, the same way as the mail secrets above. The key never goes in `site.toml`.

`khalid --site <folder> bing sitemap` hands Bing the sitemap, and `khalid --site <folder> bing report` shows Bing's last crawl day, the sitemaps it holds and the addresses it had trouble with. Both take `--json`.

## Google Search Console

Search Console is Google's report of how a site does in Google search. Khalid reads it with your own Google account, and only once you opt in. It never involves SerpDad unless you connect SerpDad yourself.

1. Add the site in Search Console and verify it, with the code Google gives you as `google` under `[seo.verification]` in `site.toml`.
2. In Google Cloud, make a project, turn on the "Google Search Console API", and make an OAuth client ID of type "Desktop app" under APIs and Services, Credentials. Google shows a client ID and a client secret.
3. Get a refresh token, a long-lived permission for that client to read your numbers: in the [OAuth 2.0 Playground](https://developers.google.com/oauthplayground/), open the settings, tick "Use your own OAuth credentials" and enter the client ID and secret, then authorise the scope `https://www.googleapis.com/auth/webmasters.readonly` with the Google account that owns the site and exchange the code for tokens. Copy the refresh token.
4. Add the table to `site.toml`. `property` is needed only when it differs from `address`, such as a domain property. `report_to` is who gets the weekly email, and needs `[mail]`.

```toml
[search_console]
client_id = "1234-abcd.apps.googleusercontent.com"
property = "sc-domain:example.com"
report_to = "you@example.com"
```

5. Set `KHALID_GOOGLE_CLIENT_SECRET` and `KHALID_GOOGLE_REFRESH_TOKEN` in the environment, the same way as the mail secrets above. Neither ever goes in `site.toml`.

Then, all with `--json` when a program reads the answer:

- `khalid --site <folder> search-console check` signs in and says whether the property can be read.
- `khalid --site <folder> search-console report` shows each post's clicks, impressions, click-through rate and position for the last 28 days Google has final numbers for, posts whose clicks fell three months in a row, missing and noindex pages that still get search traffic, and the week's winners and losers.
- `khalid --site <folder> search-console post <slug>` shows one post's top searches and whether Google has it in its index.
- `khalid --site <folder> search-console index` asks Google about every published post, up to 200 a run. Google allows 2,000 of these a day.
- `khalid --site <folder> search-console follow-ups` lists the long searches, 8 words or more, that showed each post in the last 28 days. People ask AI assistants in whole questions like these, and each is one a reader brings to the post. Next to each, it names the heading that answers it, or says no heading does yet. The fix is a heading that asks the question, with a 40 to 60 word answer under it. Google leaves out searches too rare to share, so a small site may see none.
- `khalid --site <folder> search-console email` sends the weekly email. Run it once a week, for example from a systemd timer with `OnCalendar=Mon 08:00`, or a cron line such as `0 8 * * 1`.

`khalid serve` reads the connection and the two secrets once, at start, so restart it after setting them. It then answers the same follow-ups at `GET /api/admin/v1/search-console/follow-ups/` for a key with `settings:write`, and the admin shows them under Search Console. The posts are read on each request, so a heading you just added counts.

A refresh token stops working when you take Khalid's permission back in your Google account, or when Google lets it lapse. The commands then say so: make a new one with step 3.

## Pages AI bots read but never cite

`khalid serve` counts each page view by where the reader came from, and logs which bots read which page. `khalid --site <folder> analytics uncited` lists the pages AI bots read in the last 30 days while no reader arrived from an AI assistant, such as ChatGPT or Perplexity, in the same days. Those pages are known to the assistants but not pointed to. `--days 7` looks at a shorter period, up to the 90 days the crawl log keeps, and `--json` prints the list for other tools.

A bot reading a page never counts as a citation: only a reader sent by an AI assistant does. Bots are named by the user agent they send, which anyone can copy, so a name in the list is what the bot claimed to be. A site built as static files has no counts, so the list stays empty.

## Git mode

Git mode keeps the site in a git repository, so posts ship like code. Each draft is a branch: `khalid new "Title" --branch` starts the branch `post/<slug>`, writes the post and commits it there. Write the post, commit, push the branch and open a pull request. Merging it publishes the post.

The server then needs what was merged. Run `khalid pull` from a deploy step, or let the git host tell the server on every push:

1. Make a long random secret, for example with `openssl rand -hex 32`, and set it as `KHALID_GIT_HOOK_SECRET` in the server's environment, the same way as the mail secrets above. The secret never goes in `site.toml`.
2. On GitHub, add a webhook under Settings, Webhooks, with the payload address `https://<your host>/api/hooks/git`, the content type `application/json` and the same secret. On GitLab, add it under Settings, Webhooks, with the same address and the secret as its token, for push events.

The server answers a signed push at once and pulls in the background; the post shows as soon as the pull finishes. A push signed with anything else gets a 401 and changes nothing. Without `KHALID_GIT_HOOK_SECRET`, the address does not exist.

## Analytics in the admin

The Analytics screen in the admin shows the same counts for the last 7, 28 or 90 days: the page views against the days before, each day, where readers came from, the most read pages, the readers each AI assistant sent, and the pages AI bots read but never cite. It needs an owner or admin sign-in, because views are visitor data.

Programs read the same numbers from the Admin API with a key that has `settings:write`: `GET /api/admin/v1/analytics/visitors/?days=28`, `GET /api/admin/v1/analytics/ai-visitors/?days=28` and `GET /api/admin/v1/analytics/uncited/?days=30`. The OpenAPI document at `/api/openapi.json` describes each answer.

## Photo and GIF search in the editor

Writers can find a free photo on Unsplash or a GIF on GIPHY from the editor's block menu. Both are off until you turn them on in `site.toml`:

```toml
[media]
unsplash = true
gifs = true
```

Each needs a key, the password the blog uses to ask for you. Make an app at unsplash.com/developers and set its access key as `KHALID_UNSPLASH_ACCESS_KEY`, and make one at developers.giphy.com and set its key as `KHALID_GIPHY_API_KEY`, the same way as the mail secrets above. The keys never go in `site.toml` and never reach the writer's browser. Run `khalid doctor` afterwards: its "Picture search" line says which searches are on and names any key the server cannot see. It only checks that the key is set; it does not send it to Unsplash or GIPHY.

A chosen photo or GIF loads from Unsplash or GIPHY, as their terms ask, so readers' browsers fetch that picture from there. A photo gets its photographer's name and links under it.
