### Added

- The newsletter's confirmation email, in English and German with English for any other language: it names the site, holds the link that confirms within two days and a link that removes the address at once, both under the site's mount path. `khalid_newsletter::Mailer` is the trait a mail transport implements, and `MemoryMailer`, behind the `test-util` feature, is the fake transport tests use (task 5.5, KH-GRO-2).
