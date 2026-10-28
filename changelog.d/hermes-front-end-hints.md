### Added

- Every built page carries its referrer policy as a meta tag, for hosts that drop headers, and `khalid serve` answers HTML with a `Link` header that says to preload the stylesheet, which an HTTP stack can send ahead as a 103 Early Hints (G20, first part).
