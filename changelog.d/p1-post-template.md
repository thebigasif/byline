### Added

- A post with `template: wide` is drawn with `theme/templates/post-wide.html` in place of `post.html`. A theme may now hold `post-<name>.html` files besides the defaults it replaces, and a post that asks for a template the theme lacks stops the build with a message naming both files (KH-EDT-3).
