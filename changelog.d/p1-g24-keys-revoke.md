### Added

- `DELETE /api/admin/v1/keys/{id}/` revokes an API key, which stops working at once; the key making the request is kept, so nobody locks themselves out (KH-API-2).
