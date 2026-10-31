### Added

- The honest update switch on Admin API edits: `real_update` with a short note sets a live post's `updated` date to today, and an edit that leaves the post's text as it was, or touches a post that is not live, is refused with a plain reason. Without the switch the date never moves. Posts in Admin API shape now show `updated` and `updated_note`.
