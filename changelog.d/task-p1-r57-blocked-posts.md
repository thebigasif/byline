### Changed

- A post the linter blocks no longer stops the whole build. `khalid build` and `khalid export` leave that post out, write the rest of the site, and list what they left out and why.
- `khalid serve` leaves out the same posts, so a running site never shows a post the build would refuse.
