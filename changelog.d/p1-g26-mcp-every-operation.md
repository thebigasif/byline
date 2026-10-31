### Changed

- `khalid mcp` offers a tool for every API operation a key may call, made from the API's OpenAPI document, so a new route is a new tool with no extra code. Tools are named by `operationId`: `list_posts`, `read_post`, `create_draft` and `edit_post` are now `browse_posts`, `read_admin_post`, `create_admin_post` and `edit_admin_post`; `publish_post` and `lint_post` stay. The staff and webhook tools reach the site's own database. A test fails when an operation has no tool (KH-API-5).
