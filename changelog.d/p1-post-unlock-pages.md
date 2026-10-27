### Added

- `khalid serve` answers a password post's form at `<mount>_unlock/`: a right password sets a cookie that opens that post for 30 days and sends the reader back to it; a wrong one shows the page again with its alert, status 401. The full page is kept at a hidden address that answers 404 from outside, and is sent only with the cookie, never cached by a shared cache. Ten wrong tries a minute per post at most (task 5.11).
