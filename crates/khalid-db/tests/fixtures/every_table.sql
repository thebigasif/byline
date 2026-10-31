-- One row in every table for two sites, so rebuild tests can see what a
-- rebuild of site 1 empties, what it keeps, and that site 2 is untouched. A
-- test fails when a table in the schema has no row here.

INSERT INTO sites (id, handle) VALUES (1, 'main'), (2, 'sister');

INSERT INTO authors (id, site_id, locale, slug, name, path, content_hash) VALUES
    (1, 1, 'en', 'asif', 'Asif', 'content/en/authors/asif.md', zeroblob(32)),
    (2, 2, 'en', 'asif', 'Asif', 'content/en/authors/asif.md', zeroblob(32));
INSERT INTO categories (id, site_id, locale, slug, name) VALUES
    (1, 1, 'en', 'guides', 'Guides'), (2, 2, 'en', 'guides', 'Guides');
INSERT INTO tags (id, site_id, locale, slug, name) VALUES
    (1, 1, 'en', 'links', 'Links'), (2, 2, 'en', 'links', 'Links');
INSERT INTO posts (id, site_id, locale, collection, slug, path, title, date, content_hash) VALUES
    (1, 1, 'en', 'posts', 'old-post', 'content/en/posts/old-post.md', 'Old post', '2025-01-02', zeroblob(32)),
    (2, 2, 'en', 'posts', 'old-post', 'content/en/posts/old-post.md', 'Old post', '2025-01-02', zeroblob(32));
INSERT INTO post_tags (site_id, post_id, tag_id) VALUES (1, 1, 1), (2, 2, 2);
INSERT INTO pages (id, site_id, locale, slug, path, title, content_hash) VALUES
    (1, 1, 'en', 'about', 'content/en/pages/about.md', 'About', zeroblob(32)),
    (2, 2, 'en', 'about', 'content/en/pages/about.md', 'About', zeroblob(32));
INSERT INTO collection_entries (id, site_id, locale, collection, slug, path, title, content_hash) VALUES
    (1, 1, 'en', 'glossary', 'anchor-text', 'content/en/glossary/anchor-text.md', 'Anchor text', zeroblob(32)),
    (2, 2, 'en', 'glossary', 'anchor-text', 'content/en/glossary/anchor-text.md', 'Anchor text', zeroblob(32));
INSERT INTO translations (site_id, key, locale, path) VALUES
    (1, 'old-post', 'en', 'content/en/posts/old-post.md'), (2, 'old-post', 'en', 'content/en/posts/old-post.md');
INSERT INTO links (site_id, from_path, line, target, to_path, anchor_text) VALUES
    (1, 'content/en/posts/old-post.md', 3, '/blog/about/', 'content/en/pages/about.md', 'about us'),
    (2, 'content/en/posts/old-post.md', 3, '/blog/about/', 'content/en/pages/about.md', 'about us');
INSERT INTO lint_results (site_id, path, rule, severity, line, message) VALUES
    (1, 'content/en/posts/old-post.md', 'no-description', 'warn', NULL, 'This post has no description.'),
    (2, 'content/en/posts/old-post.md', 'no-description', 'warn', NULL, 'This post has no description.');
INSERT INTO alias_redirects (site_id, from_path, to_path) VALUES
    (1, '/2025/old-post/', 'content/en/posts/old-post.md'), (2, '/2025/old-post/', 'content/en/posts/old-post.md');
INSERT INTO search_pages (id, site_id, locale, path, href, title, body, title_words, heading_words, body_words) VALUES
    (1, 1, 'en', 'content/en/posts/old-post.md', '/blog/old-post/', 'Old post', 'Words.', 2, 0, 1),
    (2, 2, 'en', 'content/en/posts/old-post.md', '/blog/old-post/', 'Old post', 'Words.', 2, 0, 1);
INSERT INTO search_terms (site_id, term, page_id, in_title, in_headings, in_body) VALUES
    (1, 'old', 1, 1, 0, 0), (2, 'old', 2, 1, 0, 0);

INSERT INTO redirects (site_id, from_path, to_url, status, origin, created_at) VALUES
    (1, '/old/', '/blog/old-post/', 301, 'manual', '2026-10-26T09:00:00Z'),
    (2, '/old/', '/blog/old-post/', 301, 'manual', '2026-10-26T09:00:00Z');
INSERT INTO not_found_log (site_id, path, hits, first_seen, last_seen) VALUES
    (1, '/blgo/', 4, '2026-10-20T08:00:00Z', '2026-10-26T08:00:00Z'),
    (2, '/blgo/', 4, '2026-10-20T08:00:00Z', '2026-10-26T08:00:00Z');
INSERT INTO users (id, site_id, email, name, role, created_at) VALUES
    (1, 1, 'editor@example.com', 'Ada Editor', 'editor', '2026-10-26T09:00:00Z'),
    (2, 2, 'editor@example.com', 'Ada Editor', 'editor', '2026-10-26T09:00:00Z');
INSERT INTO sessions (site_id, user_id, token_hash, created_at, expires_at) VALUES
    (1, 1, randomblob(32), '2026-10-26T09:00:00Z', '2026-11-26T09:00:00Z'),
    (2, 2, randomblob(32), '2026-10-26T09:00:00Z', '2026-11-26T09:00:00Z');
INSERT INTO passkeys (site_id, user_id, credential_id, credential, name, created_at) VALUES
    (1, 1, randomblob(16), '{"cred":{}}', 'Laptop', '2026-10-26T09:00:00Z'),
    (2, 2, randomblob(16), '{"cred":{}}', 'Laptop', '2026-10-26T09:00:00Z');
INSERT INTO magic_links (site_id, user_id, token_hash, created_at, expires_at) VALUES
    (1, 1, randomblob(32), '2026-10-26T09:00:00Z', '2026-10-26T09:15:00Z'),
    (2, 2, randomblob(32), '2026-10-26T09:00:00Z', '2026-10-26T09:15:00Z');
INSERT INTO api_keys (id, site_id, key_id, kind, name, key_hash, created_at) VALUES
    (1, 1, '0123456789abcdef', 'integration', 'Zapier', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA', '2026-10-26T09:00:00Z'),
    (2, 2, 'fedcba9876543210', 'integration', 'Zapier', '$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA', '2026-10-26T09:00:00Z');
INSERT INTO api_key_scopes (site_id, api_key_id, scope) VALUES (1, 1, 'posts:read'), (2, 2, 'posts:read');
INSERT INTO webhooks (id, site_id, event, url, secret, created_at) VALUES
    (1, 1, 'post.published', 'https://hooks.example.com/k', randomblob(32), '2026-10-26T09:00:00Z'),
    (2, 2, 'post.published', 'https://hooks.example.com/k', randomblob(32), '2026-10-26T09:00:00Z');
INSERT INTO webhook_deliveries (id, site_id, webhook_id, event, url, body, first_at, state, state_at) VALUES
    (1, 1, 1, 'post.published', 'https://hooks.example.com/k', X'7B7D', '2026-10-26T09:00:00.000000Z', 'pending', '2026-10-26T09:01:00.000000Z'),
    (2, 2, 2, 'post.published', 'https://hooks.example.com/k', X'7B7D', '2026-10-26T09:00:00.000000Z', 'delivered', '2026-10-26T09:00:00.000000Z');
INSERT INTO webhook_attempts (site_id, delivery_id, number, at, status, reason) VALUES
    (1, 1, 1, '2026-10-26T09:00:00.000000Z', 503, NULL),
    (2, 2, 1, '2026-10-26T09:00:00.000000Z', 204, NULL);
INSERT INTO post_schedules (site_id, post_id, publish_at, unpublish_at) VALUES
    (1, 'en.choosing-anchor-text', '2026-11-03T08:00:00Z', NULL),
    (2, 'en.choosing-anchor-text', NULL, '2026-12-01T00:00:00Z');
INSERT INTO site_keys (site_id, purpose, secret, created_at) VALUES
    (1, 'preview', randomblob(32), '2026-10-27T09:00:00Z'),
    (2, 'preview', randomblob(32), '2026-10-27T09:00:00Z');
INSERT INTO form_submissions (site_id, form, fields, status, created_at) VALUES
    (1, 'contact', '{"message":"Hello"}', 'received', '2026-10-26T09:00:00Z'),
    (2, 'contact', '{"message":"Hello"}', 'received', '2026-10-26T09:00:00Z');
INSERT INTO subscribers (id, site_id, email, status, created_at) VALUES
    (1, 1, 'reader@example.com', 'confirmed', '2026-10-26T09:00:00Z'),
    (2, 2, 'reader@example.com', 'confirmed', '2026-10-26T09:00:00Z');
INSERT INTO subscriber_lists (site_id, subscriber_id, list, joined_at) VALUES
    (1, 1, 'main', '2026-10-26T09:00:00Z'), (2, 2, 'main', '2026-10-26T09:00:00Z');
INSERT INTO subscriber_tokens (site_id, subscriber_id, purpose, token_hash, created_at) VALUES
    (1, 1, 'unsubscribe', randomblob(32), '2026-10-26T09:00:00Z'),
    (2, 2, 'unsubscribe', randomblob(32), '2026-10-26T09:00:00Z');
INSERT INTO comments (site_id, post_path, author_kind, author_name, author_email, body, status, created_at) VALUES
    (1, 'content/en/posts/old-post.md', 'reader', 'Ben Reader', 'ben@example.com', 'Useful, thank you.', 'approved', '2026-10-26T09:00:00Z'),
    (2, 'content/en/posts/old-post.md', 'reader', 'Ben Reader', 'ben@example.com', 'Useful, thank you.', 'approved', '2026-10-26T09:00:00Z');
INSERT INTO analytics_counts (site_id, day, path, referrer_group, views) VALUES
    (1, '2026-10-26', '/blog/old-post/', 'search', 12), (2, '2026-10-26', '/blog/old-post/', 'search', 12);
