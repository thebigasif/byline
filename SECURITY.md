# Security

Khalid is a public-facing server. We treat security reports as the highest priority work in the repository.

## Reporting

Please report vulnerabilities privately through GitHub's "Report a vulnerability" button on this repository, or by email to security@serpdad.com. Do not open a public issue.

Include what you found, how to reproduce it, and what version or commit you tested. We will acknowledge within three working days and keep you informed until the fix ships.

## What we do

- Dependencies are audited on every build with `cargo deny`, and Dependabot opens pull requests for updates.
- The rules under "Security" in `docs/standards/RULES.md` are enforced by tests and lints: sanitised HTML, Content Security Policy with nonces, Argon2id passwords, constant-time token checks, path containment, rate limits, content-sniffed uploads, signed webhooks and an outbound client that refuses private network addresses.
- A fix for a confirmed vulnerability is released as a patch version with a changelog entry that credits the reporter, if they wish.

## Supported versions

Until 1.0, only the latest release is supported.
