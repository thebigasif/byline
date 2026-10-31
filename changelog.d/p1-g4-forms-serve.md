### Added

- Forms work on `khalid serve`. A form block on a post or page now posts to `<mount>forms/<id>`, the reader sees a thank-you page, and the message is kept in the site database. Robots that fill the hidden trap are dropped, and one address that sends too many is asked to wait (G4, KH-GRO-1). Sending messages on by email or webhook comes with G40.
