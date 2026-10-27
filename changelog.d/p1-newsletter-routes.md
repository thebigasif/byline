### Added

- `khalid_newsletter::router`, the newsletter's addresses on a running site: the sign-up form posts to `subscribe` under the mount path, and to `/subscribe`, where the `::newsletter` block's form posts. A confirmation or unsubscribe link opens a page with one button, and a mail program's one-click post unsubscribes at once (RFC 8058). A wrong address shows the form again, a filled trap field sends nothing, and a site that cannot send email says sign-ups are not open. Pages are never cached, framed or sent on as a referrer (task 5.5, KH-GRO-2).
