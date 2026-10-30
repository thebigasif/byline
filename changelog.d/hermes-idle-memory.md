### Changed

- The server's idle memory sits under its budget again: freed pages return to the OS at once (`MIMALLOC_PURGE_DELAY=0`), set everywhere Khalid runs, in the deployment image, the systemd unit, `compose.yaml`, and named for hand-run servers in the install docs. The database's page caches are also capped at 4 MB for the writer and 2 MB for each of the eight readers, so they no longer drift with the host's memory. Both are recorded in the performance standard with their CI measurements.
