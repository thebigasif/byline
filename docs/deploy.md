# Running Khalid on a server

Khalid is one file. Put it on a machine, point it at a site folder, and it serves the blog and the admin. This page shows the ways to keep it running. The files it names are in `packaging/` in the repository.

A site folder is the folder that holds `site.toml`, `content/` and `theme/`. Khalid keeps what it works out from those files, such as its database and image sizes, in `data/` inside the same folder. Back up the site folder and you have backed up the blog.

## Install with one line

On Linux or a Mac with Apple silicon:

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

Khalid sends email when a reader signs up for the newsletter: one message with a link that confirms the address. Until `site.toml` says how mail is sent, the sign-up form says sign-ups are not open yet and stores nothing.

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

## Bing Webmaster Tools

Bing Webmaster Tools is Bing's dashboard for site owners. Add the site there with the address in `site.toml`, put the code Bing gives you as `bing` under `[seo.verification]` in `site.toml`, and verify. Then make an API key, the password a program uses to reach Bing for you, under Settings, API Access, and set it as `KHALID_BING_API_KEY` in the environment, the same way as the mail secrets above. The key never goes in `site.toml`.

`khalid --site <folder> bing sitemap` hands Bing the sitemap, and `khalid --site <folder> bing report` shows Bing's last crawl day, the sitemaps it holds and the addresses it had trouble with. Both take `--json`.
