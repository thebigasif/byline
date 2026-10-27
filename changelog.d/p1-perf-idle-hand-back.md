### Changed

- `khalid serve` hands the memory its start and its renders freed back to the system about two seconds after the work ends, instead of whenever an idle thread happened to end, so a quiet server holds less (task 1.21).
