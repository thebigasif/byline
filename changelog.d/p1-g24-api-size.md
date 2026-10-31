### Changed

- The release build optimises `khalid-api` for size, as it does the share image and import stacks: the Admin and Content APIs never serve a reader's page, and the binary stays under its 30 MB budget with the new Admin API routes.
