### Added

- `khalid_newsletter::SubscriberStore`, the trait the database implements for subscribers: record a sign-up with its confirmation token, and confirm with a link that works once for two days. A second confirmation email to the same address waits ten minutes (task 5.5, KH-GRO-2).
