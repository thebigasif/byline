### Added

- `khalid doctor` fetches the blog at its public address, the site's address with the mount such as `https://www.example.com/blog/`, and confirms Khalid answers there. That proves the proxy rule on the main site works. It tells apart an address that does not answer, an error such as 404, the main site answering instead of the blog, and a private address it will not fetch, and says what to do for each. `--offline` skips the fetch, for a site that is not live yet (KH-INS-3, KH-OPS-2).
