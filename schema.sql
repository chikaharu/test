CREATE TABLE IF NOT EXISTS orders (
    id BIGSERIAL PRIMARY KEY,
    order_id TEXT NOT NULL UNIQUE,
    postal_code TEXT NOT NULL,
    address TEXT NOT NULL,
    cart_items JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('accepted', 'hold')),
    hold_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
