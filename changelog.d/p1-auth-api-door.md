### Added

- The Admin API accepts a signed-in staff session as well as a staff token, but only when the request also carries the session's CSRF token in the `x-khalid-csrf` header, so another site cannot act for a signed-in person (BL-SEC-9, KH-API-2).
