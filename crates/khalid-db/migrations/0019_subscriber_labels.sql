-- Subscriber labels (G39). A label is a slug such as paid or from-launch;
-- a segment is one newsletter's confirmed subscribers who carry certain
-- labels. Visitor data: a rebuild never touches it.
CREATE TABLE subscriber_labels (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    subscriber_id INTEGER NOT NULL REFERENCES subscribers (id) ON DELETE CASCADE,
    label TEXT NOT NULL CHECK (length(label) > 0),
    added_at TEXT NOT NULL CHECK (julianday(added_at) IS NOT NULL),
    PRIMARY KEY (subscriber_id, label)
) STRICT, WITHOUT ROWID;

CREATE INDEX subscriber_labels_by_label ON subscriber_labels (site_id, label);
