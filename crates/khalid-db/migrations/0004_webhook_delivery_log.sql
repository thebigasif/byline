-- The webhook delivery log in the shape khalid-webhooks gives it (task 1.15):
-- the event, the address and the exact bytes sent, every attempt, and where
-- the delivery stands. Nothing has written to webhook_deliveries yet, so it is
-- replaced rather than altered.
DROP INDEX webhook_deliveries_due;
DROP TABLE webhook_deliveries;

-- Times are UTC, written as 2026-10-26T09:00:00.000000Z, so text order is
-- time order and the index answers "what is due" without a sort.
CREATE TABLE webhook_deliveries (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    webhook_id INTEGER NOT NULL REFERENCES webhooks (id) ON DELETE CASCADE,
    event TEXT NOT NULL,
    url TEXT NOT NULL,
    body BLOB NOT NULL,
    first_at TEXT NOT NULL CHECK (julianday(first_at) IS NOT NULL),
    state TEXT NOT NULL CHECK (state IN ('pending', 'delivered', 'gave_up')),
    -- When a pending delivery is due, or when it was delivered or given up.
    state_at TEXT NOT NULL CHECK (julianday(state_at) IS NOT NULL)
) STRICT;

-- The retry timer reads the pending deliveries that are due, oldest first.
CREATE INDEX webhook_deliveries_due ON webhook_deliveries (site_id, state_at, id)
    WHERE state = 'pending';

-- The admin's log of one webhook, newest first.
CREATE INDEX webhook_deliveries_by_webhook ON webhook_deliveries (webhook_id, id);

-- Each attempt has a status when the receiver answered, or a reason when it
-- could not be reached, never both.
CREATE TABLE webhook_attempts (
    site_id INTEGER NOT NULL REFERENCES sites (id) ON DELETE CASCADE,
    delivery_id INTEGER NOT NULL REFERENCES webhook_deliveries (id) ON DELETE CASCADE,
    number INTEGER NOT NULL CHECK (number >= 1),
    at TEXT NOT NULL CHECK (julianday(at) IS NOT NULL),
    status INTEGER CHECK (status BETWEEN 100 AND 599),
    reason TEXT,
    CHECK ((status IS NULL) <> (reason IS NULL)),
    PRIMARY KEY (delivery_id, number)
) STRICT, WITHOUT ROWID;
