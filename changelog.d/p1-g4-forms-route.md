### Added

- `khalid_forms::form_router` answers `POST <mount>forms/<id>`: it keeps what a reader sent and shows a thank-you page, quietly drops what spam robots send through the hidden trap, slows down one address that sends too many, and says in plain words why anything else was not sent (KH-GRO-1, BL-SEC-5).
- `khalid_comments::reader_address` gives other routes the reader's address the comment limits count by.
