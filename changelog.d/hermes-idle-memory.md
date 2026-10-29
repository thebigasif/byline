### Changed

- The server's freed pages return when their thread ends: the deployment image sets `MIMALLOC_ABANDONED_PAGE_PURGE=1`, `khalid-perf` measures the server with the same setting, and the setting is recorded in the performance standard with its measurement.
