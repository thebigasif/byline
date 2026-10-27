### Added

- `khalid doctor` on a static site lists the same features `khalid build` says are off, each with why and how to turn it on, just above its last line. A static site is not a problem, so doctor still exits with 0. `--json` gives the list under `off`, the same as the build (KH-OPS-1, KH-OPS-2).
