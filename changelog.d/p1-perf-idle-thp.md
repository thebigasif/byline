### Changed

- `khalid serve` no longer uses 2 MB huge pages, which a quiet server filled around a little memory still in use and held whole: the allocator stops asking for them, and the server switches them off for itself where the system hands them out unasked (task 1.21).
