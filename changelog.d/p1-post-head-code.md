### Added

- A post's own head code, such as an analytics snippet, is checked before it is kept: only `script`, `style`, `meta` and `link` may be in it, and text, body elements, `<base>`, `<meta http-equiv>`, event attributes such as `onload`, and unclosed tags are refused with a message that names the problem. What is kept is written again with every attribute quoted (KH-EDT-3).
