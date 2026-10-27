Added
The server can send a post's images and files: PNG, JPEG, WebP, AVIF, GIF, SVG and PDF each answer with their own type, any other file as bytes to save, and `MediaType::for_file` picks the type from a file's name. Images are sent as they are, never compressed again (KH-CON-3).
