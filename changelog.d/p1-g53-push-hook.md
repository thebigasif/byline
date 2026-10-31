### Added

- Git mode's push hook: with `KHALID_GIT_HOOK_SECRET` set, `khalid serve` answers `POST /api/hooks/git` signed by GitHub or GitLab with that secret, pulls what was pushed in the background, and serves it. An unsigned push gets a 401 (KH-EDT-4).
