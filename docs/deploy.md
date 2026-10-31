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

## Sending email

Khalid sends email when a reader signs up for the newsletter: one message with a link that confirms the address. Until `site.toml` says how mail is sent, the sign-up form says sign-ups are not open yet and stores nothing.

Gated content, the part of a post inside `:::gated`, opens with the same email. A reader types their address in the form where the part would be, opens the link, clicks the button, and lands back on the post with the part open, with scripts off. Their browser keeps a cookie for 180 days that opens gated content on every post. On another phone or browser they type the same address again and get a link that opens it there. Without `[mail]` the part stays locked for everyone.

Add a `[mail]` table with the address mail comes from and exactly one way to send it.

Through the mail server your host gives you, with SMTP (the protocol mail servers speak):

```toml
[mail]
from = "news@example.com"
name = "Field Notes"

[mail.smtp]
host = "smtp.example.com"
username = "news@example.com"
```

Khalid connects on port 587 and turns encryption on before it sends anything. Add `port = 465` and `security = "tls"` for a server that is encrypted from the start. A relay on the same machine or your own network needs `private_host = true`; only then may it use `security = "none"`.

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
```

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

A super admin is a person whose email `sites.toml` lists and who signs in on the home site. They can add a site from there while the server runs, and it answers at once. Keys and staff of the other sites cannot.

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
