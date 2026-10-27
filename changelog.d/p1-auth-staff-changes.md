### Added

- Staff who manage others can change a role (`PUT /api/admin/v1/staff/{id}/`) or remove an account (`DELETE` on the same address), only within what their role may manage. The owner, signed in, can hand the site to another account (`POST /api/admin/v1/staff/{id}/owner/`) and becomes an admin (KH-EDT-7).
