# Khalid, by SerpDad

**Status: Phase 0 complete, Phase 1 starting.** Byline and Wellranked were working names; the product is Khalid.

Khalid is an open-source blog engine any business can add to its website, whatever the site is built with. One Rust binary. Every post is a text file. The best SEO defaults are built in, including schema markup for every content type and a site audit that passes the Ahrefs and Semrush checks. A linter refuses to publish a post that breaks basic SEO hygiene, and every refusal teaches why. It is run from the command line and from a web editor, by people and by agents. There is no drag-and-drop page builder.

The one-line idea: get an Ahrefs- or Semrush-quality blog in one click.

## Install

On Linux or a Mac, once the first release is out:

```sh
curl -fsSL https://raw.githubusercontent.com/thebigasif/khalid/main/scripts/install.sh | sh
khalid init blog --address https://blog.example.com --yes
khalid --site blog serve
```

The blog is then at `http://127.0.0.1:8080/`. To sign in to the admin, make yourself the owner from a second terminal:

```sh
khalid --site blog staff owner --email you@example.com --name "Your Name"
```

Khalid shows the sign-in link only this once. Open the `localhost` address it shows while `khalid serve` runs.

`docs/deploy.md` covers the systemd service, the Docker image and Docker Compose, and the ready files for Fly.io, Railway, Coolify and a Hetzner Cloud server.

## Where things are

| | |
|---|---|
| `docs/plan.md` | The build plan, with every founder decision. Read it first |
| `docs/phases/` | What is being built right now, task by task |
| `docs/standards/` | The rule catalogue and the standards every change follows |
| `docs/adr/` | Architecture decisions |
| `CLAUDE.md` | The contract for the coding agents that write this code |
| `FRONTEND.md` | The front-end checklist every template follows |
| `.claude/skills/` | Agent skills: BeautifulRust's Rust skills, plus this repository's own |
| `lints/` | The clippy configuration, and the crate that proves every lint fires |

## How it is built

The code is written by Claude Opus 5.5 and reviewed by agents against a 10-point rubric before a person sees it. The rules come from [BeautifulRust](https://github.com/morganlinton/BeautifulRust) (MIT, Morgan Linton), the Rust API Guidelines, Effective Rust and Microsoft's Pragmatic Rust Guidelines, with this repository's own rules on top. Every machine-checkable rule is a lint or a CI gate. Performance has budgets, and the budgets are gates too.

## Licence

AGPL-3.0, see `LICENSE`. The name and logo are trademarks of SerpDad, see `TRADEMARK.md`. Contributors sign the short agreement in `CLA.md` once, through a bot on their first pull request.

## Security

See `SECURITY.md`. Please do not open public issues for vulnerabilities.
