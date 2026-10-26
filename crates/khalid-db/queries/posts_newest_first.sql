SELECT p.path, p.locale, p.collection, p.slug, p.title, p.description,
    p.date AS "date: chrono::NaiveDate", p.author, p.category,
    t.key AS "translation?", p.content_hash
FROM posts AS p
LEFT JOIN translations AS t ON t.site_id = p.site_id AND t.path = p.path
WHERE p.site_id = ?1 AND p.locale = ?2 AND p.collection = ?3
ORDER BY p.date DESC, p.slug
LIMIT ?4
