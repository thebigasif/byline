### Added

- Staff passwords need at least 12 characters and are stored only as an Argon2id hash. Signing in with an email that has no account takes as long as a wrong password, so nobody can tell which emails have accounts (KH-EDT-7).
