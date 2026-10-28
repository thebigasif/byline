### Changed

- The Content API now says no to `include` and `fields` with a 400 that names the parameter, instead of accepting them and sending every field anyway (audit F6, G23). ETags and 304 answers are tested through `khalid serve` itself, not only through the library.
