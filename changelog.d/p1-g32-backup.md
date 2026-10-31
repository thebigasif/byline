### Added

- `khalid backup` writes one `.tar.gz` archive of the site folder. It leaves out the built site and the image sizes, which Khalid makes again, and never writes over a file. `--out` names the archive; without it, the name holds the folder and the time (KH-OPS-2).
