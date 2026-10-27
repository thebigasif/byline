# 0000. Khalid ships a systemd unit, a scratch image and a checked install script

**Number:** next free. **Status:** Proposed, 2026-10-27. Phase 5 task 5.14 (KH-INS-1).

## Context

Task 5.14 asks for a systemd unit, a Docker image built from scratch on a fully static musl binary, a Compose file and the install script. Plan section 4.7 says a site is one folder and one binary. The deploy answers (`docs/decisions/2026-10-27-phase-5-answers.md`, item 4) say Linux releases become fully static musl builds and that the release workflow change is proposed, not made, by the deploy track. The build server has no working Docker, so the image cannot be built here.

## Decision

- **systemd.** `packaging/systemd/khalid.service` runs `khalid --site /var/lib/khalid serve --listen 127.0.0.1:8080` as a fixed `khalid` user from `packaging/systemd/khalid.sysusers`. A fixed user, not `DynamicUser=`, so a person can edit the content files with `sudo -u khalid`. `StateDirectory=` makes the site folder; `ProtectSystem=strict` leaves it the only place the service can write. It may bind ports 80 and 443 for `serve --domain`. Blocked system calls return an error rather than kill the server.
- **Image.** The `Dockerfile` at the repository root builds the admin with Node, then Khalid on `rust:<pinned>-alpine`, which targets musl and links statically with no extra tool. The final stage is `scratch` with the binary, the CA certificates and an empty `/tmp`. A `RUN ["/khalid", "--version"]` step inside the empty image is the proof that the binary is static. It runs as user 65532 with the site at `/site` on port 8080.
- **Compose.** `compose.yaml` runs the image read-only with every capability dropped, as the host's user, on `./site`. It names `ghcr.io/thebigasif/khalid` and builds locally until that image is published.
- **Install script.** `scripts/install.sh` is POSIX `sh`. It picks the release file by `uname`, keeps the existing file names (`khalid-<tag>-linux-x86_64` and so on, so the musl switch changes no link), installs only when the file's SHA-256 matches the release's `SHA256SUMS`, and swaps the binary with one rename. `KHALID_RELEASES` points it at a folder, so tests run it without the network.
- **Tests.** Each package's own command line is read from its file and run against the demo site. Version pins in the `Dockerfile` must match `rust-toolchain.toml` and `ci.yml`.

## Consequences

- The image build and the musl release are proven only in CI, through the jobs proposed in the pull requests. Until the CTO adds them, a broken `Dockerfile` would not fail any check.
- The release workflow must build the admin before the binary, or released binaries and the image differ at `/admin/`.
- A new release platform needs a line in the install script's platform table.
