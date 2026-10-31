### Added

- Turning and cropping library pictures (G50, KH-EDT-1): `POST /api/admin/v1/media/{id}/copies/` takes a quarter turn and a crop box in thousandths and makes a new copy in the media library, with the same name, alt text and caption and the focal point moved with the picture. The original stays, so posts that show it do not change. The same edit made twice finds the first copy.
