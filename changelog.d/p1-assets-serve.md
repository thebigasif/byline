Added
The server can hold a stylesheet: `MediaType::Css` answers as `text/css`, and `Page::immutable` marks a file whose name carries a hash of its content, which readers may keep for a year without asking again (`Cache-Control: public, max-age=31536000, immutable`). Pages are still sent with `no-cache` (BL-PERF-2, KH-WEB-6).
