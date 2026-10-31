# 0000. Many sites in one process: a sites folder, routing by host, and super admins listed by email

**Number:** next free. **Status:** Proposed, 2026-10-31. Gap brief G48, KH-EDT-7, plan section 4.4, decision D-2.

## Context

D-2 says one program can serve many sites, the default install is one, and a super-admin panel adds domains to the same process. Plan 4.4 adds a `sites/` folder, routing by host name, its own users, theme, content and database per site, and a super admin who sees every site. It does not say where the list of sites lives, who a super admin is, how they sign in, or what happens to HTTPS. Every command already works on one site folder with one `data/site.db`. The founder said yes in advance (2026-10-27), so these choices are recorded here.

## Decision

- **A sites folder is a folder with `sites.toml`.** The file lists each site's domain in `[[site]]` tables and the super admins' emails in `super_admins`. Each site is an ordinary site folder in `sites/<domain>/`, so `khalid --site sites/<domain> ...` works for every other command unchanged, and each site keeps its own users, theme, content and database. `khalid init` still makes one site folder: the default install stays one site, and a folder becomes a sites folder with `khalid sites add <domain>`.
- **The first site is the home site.** The super-admin panel lives in its admin. Domains are lowercased and must be DNS names without empty labels, so a domain is always a safe folder name.
- **A super admin is a person, never a key.** They are a staff account on the home site, signed in by any of the usual ways, whose email `sites.toml` lists (`khalid sites admin <email>`). `GET` and `POST /api/admin/v1/sites/` need the session cookie and its CSRF token; a key, even an owner's staff token, gets 403. A session on another site, or the same person signed in on another site, is no super admin there. The list is read from the file on each request, so a change from the command line counts at once.
- **One listener, routed by `Host`.** `khalid serve` on a sites folder runs every site's usual serve path on one shared runtime, each on its own thread, and sends each request to the site whose whole domain it names. A host no site has gets a plain 404. A site added through the panel or `khalid sites add` while the process runs is started without a restart. A site that fails its check is reported and the others go on.
- **No built-in HTTPS for a sites folder yet.** `--domain` gets one certificate for one site; a sites folder refuses it with the reason and points to a proxy such as Caddy, which gets a certificate per domain. Built-in certificates for many domains is a later task.
- **The super-admin routes join every site's Admin API.** Sites other than the home site, and a site served alone, answer them for nobody. The served OpenAPI document is then the same everywhere, and `sdk/typescript/openapi.json` holds it.

## Consequences

- Backups, updates and exports stay per site; a sites folder backs up as one `khalid backup` per site folder.
- A super admin signs in on the home site only. Opening another site's admin needs an account on that site, which `khalid --site sites/<domain> staff owner` makes.
- The admin's Sites screen (plan 4.6) calls these two routes; it waits for its mockup and the founder's yes under G49.
- Many sites share one process's memory. Each site's idle cost adds up, which the performance budgets, measured for one site, do not cover yet.
