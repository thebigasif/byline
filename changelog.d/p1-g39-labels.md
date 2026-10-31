### Added

- Subscriber labels and segments (G39, KH-GRO-2). `khalid newsletter label <email> <label>` gives a subscriber a label and `--remove` takes it away; `khalid newsletter segment --list <name> --label <label>` counts the confirmed subscribers of one newsletter who carry every label named. Labels are visitor data in `data/site.db`, added by forward migration 0019.
