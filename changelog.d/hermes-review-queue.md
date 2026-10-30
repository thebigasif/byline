### Added

- A review queue (F3): a `reviewed:` date in a post's front matter records that someone looked it over without touching its `date` or `updated`, `khalid lint` warns when a post has gone six months without a review (decision D4's limit, `review-months` in `lint.toml`), and the Admin API's post rows carry `reviewed` and `review_due` so the dashboard can filter `review:due`.
