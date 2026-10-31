### Added

- Presence and post locking in the editor (G50, KH-EDT-2): the first writer to open a post holds it, and anyone else who opens it reads it, sees who is writing, and can take over after the screen says what that costs. The lock lets go when the holder closes the post, or a minute after their editor stops checking in. `POST` and `DELETE /api/admin/v1/posts/{id}/presence/` check in and leave; the lock lives in memory, so a restart frees every post.
