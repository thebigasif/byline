### Added

- Connect to SerpDad: a page in the admin that makes the key SerpDad sends drafts with, shows it once beside the blog's address, and lists the last 20 drafts SerpDad sent with their checks. The Admin API answers it at `/api/admin/v1/serpdad/`: `GET` reads the connection and the drafts, `POST` makes a new key and revokes the old one, `DELETE` turns the connection off. Drafts made with the SerpDad key are recorded in the new `serpdad_drafts` table (KH-API-6).
