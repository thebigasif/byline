### Added

- Templates that run Khalid on Fly.io (`fly.toml`), Railway (`railway.json`), Coolify (`packaging/coolify/compose.yaml`) and a Hetzner Cloud server (`packaging/hetzner/cloud-config.yaml`), with steps for each in `docs/deploy.md` (G34).

### Changed

- The image starts with `serve --init`, so it makes the site on an empty disk at the address in `KHALID_ADDRESS`.
