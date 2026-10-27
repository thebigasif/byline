### Fixed

- `khalid lint` and the publish gate now read a post's `focus_keyword` and `secondary_keywords`, its forms and its wikilinks as written. The focus keyword warnings (missing from the title, description, first paragraph, a subheading, the address or an image description, used too rarely or too often) and the unused secondary keyword warning now fire. A form whose answers go nowhere gets `form-without-destination`, and a draft with a wikilink to no page gets `broken-wikilink` in the Admin API and MCP lint results (KH-LNT-2, KH-LNT-4).
