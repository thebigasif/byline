# 0000. Templates for Fly.io, Railway, Coolify and Hetzner start from an empty disk

**Number:** next free. **Status:** Proposed, 2026-10-31. Plan 4.7, gap brief G34 (KH-INS-1).

## Context

Plan 4.7 promises one-click templates for Fly.io, Railway, Coolify and a Hetzner server. Task 5.14 shipped the systemd unit, the scratch image, Compose and the install script, but no templates. A container host starts the image on an empty disk, and the image has no shell to run `khalid init` before `serve`. None of these hosts can be tried from the build server, which has no Docker and no accounts.

## Decision

- **First start in the binary.** `khalid serve --init` makes the site with `init`'s defaults when the folder has no `site.toml`, at the address in `KHALID_ADDRESS`, and does nothing when a site is there. The image's `CMD` carries `--init`, so every container host needs only the address. No address on an empty disk stops with a message that names the variable.
- **Where the files live.** `fly.toml` and `railway.json` sit at the repository root, where `fly launch` and Railway look without being told. The Coolify file is `packaging/coolify/compose.yaml`, because the root `compose.yaml` runs on `./site` for a person's own machine. The Hetzner file is a cloud config at `packaging/hetzner/cloud-config.yaml`, because Hetzner has no app template for third parties; a cloud config pasted at server creation is its one-step path.
- **Addresses.** Coolify and Railway make the address (`SERVICE_URL_KHALID_8080`, `RAILWAY_PUBLIC_DOMAIN`). Fly does not pass its app's address to the container, so `fly.toml` holds it next to the app name and a test keeps the two equal.
- **Users.** Fly and Coolify give the volume to the image's user 65532. Railway mounts volumes as root, so its steps set `RAILWAY_RUN_UID=0` rather than making the image itself run as root.
- **Hetzner.** The server runs the systemd unit with a drop-in that serves HTTPS for the domain with `--accept-terms`, since no proxy sits in front. The domain is written once, in `/etc/khalid/domain.env`.
- **Railway button.** A Railway deploy button needs a template published from a Railway account. The guide gives the five steps until the founder publishes one; the button then goes in the README.

## Consequences

- The tests read each template and check it against the `Dockerfile` and the unit: port, volume, address, build file, and the Hetzner `init` line run as written. Whether each host accepts the file is proven only by a real deploy.
- The Hetzner path needs a published release, as the install script does.
- A change to the image's port or site folder fails the template tests until every template follows.
