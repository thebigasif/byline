### Added

- `khalid serve` counts page views on the server, with no cookies and no script, so readers with scripts blocked are counted too. Each view is kept only as the day, the page and where the reader came from, with ChatGPT, Perplexity, Gemini, Copilot and Claude grouped as AI search. Bots are not counted as readers: each bot's visit goes to the crawl log with the page and the status it got. A HEAD request, a browser fetching ahead, and an error page are not views (KH-GRO-4).
