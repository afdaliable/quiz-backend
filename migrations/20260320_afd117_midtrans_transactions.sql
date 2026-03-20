-- AFD-117: Midtrans transactions table
-- Additive — does not touch existing payment_transactions (Mayar) or user_subscriptions tables

CREATE TABLE IF NOT EXISTS midtrans_transactions (
    id                      CHAR(36)     NOT NULL PRIMARY KEY,
    user_id                 CHAR(36)     NOT NULL,
    subscription_id         INT          NULL,          -- set after subscription is activated
    plan_id                 INT          NOT NULL,
    order_id                VARCHAR(100) NOT NULL UNIQUE,
    amount                  BIGINT       NOT NULL,      -- dalam Rupiah
    status                  VARCHAR(20)  NOT NULL DEFAULT 'pending',
    -- 'pending' | 'success' | 'failed' | 'expired' | 'cancelled'
    payment_method          VARCHAR(50)  NULL,
    payment_url             TEXT         NULL,          -- Midtrans redirect URL
    snap_token              TEXT         NULL,          -- untuk Snap.js embed
    paid_at                 TIMESTAMP    NULL,
    expires_at              TIMESTAMP    NULL,          -- batas waktu bayar (24 jam)
    midtrans_transaction_id VARCHAR(100) NULL,
    midtrans_status_code    VARCHAR(10)  NULL,
    invoice_number          VARCHAR(50)  NULL,          -- INV-2026-001
    created_at              TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (plan_id) REFERENCES premium_plans(id),
    INDEX idx_mt_user_id  (user_id),
    INDEX idx_mt_order_id (order_id),
    INDEX idx_mt_status   (status)
);
