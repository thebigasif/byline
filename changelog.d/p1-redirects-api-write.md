Added
`POST /api/admin/v1/redirects/` stores a 301, 302, 307, 410 or 451, exact or regex. A redirect that would close a loop or take an address that already has one is refused with a plain message (KH-SEO-8).
