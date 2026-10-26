### Fixed

- Every decision record in `docs/adr/` now has its own number, and the index lists each one once, in order. The lint engine record moved from 0011 to 0019 and the theme tokens record from 0013 to 0020, and thirteen records that waited for a number got 0021 to 0033. Tests now refuse a number shared by two records and an index that misses or repeats one.
