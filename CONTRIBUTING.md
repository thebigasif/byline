# Contributing

Thank you for looking. Khalid is close to its first release, and most code is written by coding agents against `CLAUDE.md`. People are welcome too, and the same rules apply to everyone.

## Before you start

1. Read `docs/plan.md` for what we are building and what is out of scope.
2. Read `docs/standards/RULES.md`. Reviews cite its IDs.
3. Look for an open issue or a task in `docs/phases/`. Comment on it before starting, so two people do not build the same thing.

## Build Khalid from source

This is also how to get Khalid before its first release. You need:

- git.
- Rust, through [rustup](https://rustup.rs/). The repository pins the version in `rust-toolchain.toml`, and rustup fetches it on the first build.
- A C compiler: `build-essential` on Debian or Ubuntu, the Xcode Command Line Tools on a Mac.
- Node.js 22 with npm, for the admin.

Then, from a terminal:

```sh
git clone https://github.com/thebigasif/khalid.git
cd khalid
cd admin
npm ci
npm run build
cd ..
cargo install --path crates/khalid --locked
khalid --version
```

`npm ci` installs the admin's packages exactly as `admin/package-lock.json` lists them. `npm run build` writes the admin to `admin/dist/`. `cargo install` then builds the binary with the admin inside it and puts `khalid` on your path. To build without installing, run `cargo build --release -p khalid`; the binary is `target/release/khalid`.

Cargo alone builds Khalid without the admin. It does not stop or warn: the blog works, but every `/admin/` address answers with a page that says the admin is not built. If that happens, run the two npm commands above, then build again.

After `git pull`, run `npm ci` and `npm run build` again before you build, so the admin matches the code.

`docs/getting-started.md` takes it from there: make a site, write a post and publish it.

## How the work runs

`docs/process/build-pipeline.md` explains how tasks become pull requests and how pull requests land on `main`, for people and for coding agents you run yourself.

## The short version

- One task, one branch, one pull request, under 400 changed lines.
- Write the failing test first.
- Run `just check` before you push. CI runs the same checks.
- Fill the pull request template, including the rubric in `docs/standards/review.md`.
- Plain language in everything a person reads, and no em dashes anywhere.

## Contributor licence agreement

Your first pull request will ask you to sign `CLA.md` through a bot. It takes one click and you sign once. It lets SerpDad keep using every contribution in its own products, which is what keeps the project funded.

## Reporting a bug

Use the bug template. Say what you did, what you expected, what happened, and the version from `khalid --version`. Never paste a key, token or password.

## Proposing a rule

Every rule in `RULES.md` needs an ID, a published source and an enforcement. Open an issue with those three things and an example of the code it would catch.
