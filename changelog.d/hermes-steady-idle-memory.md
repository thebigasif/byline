### Changed

- The idle memory reading waits for `khalid serve` to settle, then takes five readings one second apart and uses the lowest, so the allocator's own schedule for handing pages back cannot swing the number between runs.
