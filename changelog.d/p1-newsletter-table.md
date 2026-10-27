### Added

- Subscribers are kept in the site's database: migration `0007_subscriber_tokens.sql` adds which newsletters each subscriber gets, the confirmation and unsubscribe links sent to them as hashes of their tokens, and when they left. `khalid_db::SubscriberTable` keeps them for `khalid-newsletter`, one site at a time (task 5.5, KH-GRO-2).
