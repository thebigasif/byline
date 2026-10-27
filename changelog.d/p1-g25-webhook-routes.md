### Added

- The Admin API lists the site's webhooks under `/api/admin/v1/webhooks/`, without their secrets, and switches one off with `DELETE /api/admin/v1/webhooks/{id}/`, keeping its delivery log. Both need `webhooks:manage` (KH-API-2, KH-API-3).
