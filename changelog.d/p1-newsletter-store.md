### Added

- `khalid_newsletter::SubscriberStore`, the trait the database implements for subscribers: record a sign-up with its confirmation token, confirm with a link that works once for two days, add an unsubscribe link for an email about to go out, and unsubscribe in one click. A second confirmation email to the same address waits ten minutes (task 5.5, KH-GRO-2).
