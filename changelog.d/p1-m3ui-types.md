### Added

- `khalid_api::follow_ups` holds the shape of Search Console follow-ups and the `FollowUpFinder` trait the Admin API reaches them through (GEO M3).
- Two problem types: `not-set-up` (409) when the site has not set up what a request needs, and `upstream` (502) when a service Khalid asked, such as Google, gave no usable answer.
