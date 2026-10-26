# Front-end checklist

Every default template follows this list. CI checks what a machine can check. The linter and `byline doctor` warn when a user's override breaks it. The list is pinned to named standards and reviewed once per release.

## Semantic HTML

- The right element for every part of the page: `header`, `nav`, `main`, `article`, `aside`, `footer`, `time` for dates, `figure` and `figcaption` for images, `address` for the author.
- Exactly one `h1`. Headings in order with no skipped levels.
- Landmarks are labelled. `lang` and `dir` are set on `html`.
- Valid HTML, checked with the W3C validator. Zero axe violations.

## Works with JavaScript blocked

- Every page renders complete on the server.
- Navigation, pagination, the language switcher, search, comment posting and the newsletter form are plain links and HTML forms first.
- JavaScript only adds polish: live search results, a copy button, hover previews. Remove it and the page is whole.
- No `noscript` notice asking to turn JavaScript on. No form without an `action`. No link that goes only to `#`.

## Responsive on every device

- Mobile first. Breakpoints only where the content needs them, never at device sizes.
- Fluid type and spacing with `clamp()` from 320 px to 1920 px. Line length between 60 and 75 characters.
- No horizontal scroll at any width from 320 px up. Long words and URLs wrap. Tables and code blocks scroll inside their own box. Embeds keep their aspect ratio.
- Images carry `srcset`, `sizes`, `width` and `height`.
- Touch targets at least 44 by 44 px with space between them. Nothing depends on hover.
- Correct viewport meta, safe-area insets, layouts that work at 200% zoom and in landscape.
- Container queries for components used in more than one place.
- Checked in CI at 320, 375, 414, 768, 1024 and 1440 px.

## Accessible

- WCAG 2.2 AA. Contrast checked against the synced theme tokens, not assumed.
- A skip link. Visible focus on everything. Full keyboard use. Native elements before ARIA.
- Every form field has a label and a plain error message. Every image has alt text or is marked decorative.
- `prefers-reduced-motion` respected. Nothing autoplays.

## Fast and stable

- Zero layout shift: reserved image space, font fallbacks tuned with `size-adjust`, no late-loading banners.
- Fonts with `font-display: swap`, self-hosted, subset to the site's languages, at most two families.
- The hero image gets `fetchpriority="high"`. Everything below the fold is lazy.
- One CSS file under 20 KB compressed. No CSS framework. No render-blocking JavaScript. No third-party script by default.
- Hashed file names with long cache lifetimes, Brotli, early hints where supported.

## Robust

- Works in reader mode, in RSS readers and when printed, with a print stylesheet.
- Content Security Policy with nonces, no inline event handlers, integrity hashes on anything external, a strict referrer policy.
- Right-to-left layouts mirror correctly. Dates, numbers and quotes follow the page language.
