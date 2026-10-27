### Added

- A media library in the Admin API under `/api/admin/v1/media/`: upload images, video, audio and files (checked by their content, with a size limit per kind and the same file never kept twice), file them in folders, give each alt text, a caption and a focal point, and list them with `filter`, `order`, `limit` and `page`. Every route needs a key with `media:write`.
