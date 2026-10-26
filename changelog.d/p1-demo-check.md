### Added

- `scripts/demo-check.sh` serves the demo site and checks every page with the W3C HTML validator, Lighthouse and axe, then again with scripts off at six widths from 320 to 1920 px. Any validator error, axe problem, sideways scroll or Lighthouse score under 100 fails it (KH-WEB-1 to KH-WEB-5, KH-WEB-7).
