### Changed

- The allocator no longer asks Linux for 2 MB huge pages, which an idle `khalid serve` filled around a little memory still in use and held whole (task 1.21).
