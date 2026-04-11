-- =====================================================
-- AFD-138: Level & XP System
-- =====================================================

-- Tabel riwayat transaksi XP
CREATE TABLE xp_transactions (
    id          VARCHAR(36)  NOT NULL DEFAULT (UUID()),
    user_id     VARCHAR(255) NOT NULL,
    amount      INT          NOT NULL,
    source      VARCHAR(50)  NOT NULL,
    source_id   VARCHAR(36)  NULL,
    description VARCHAR(200) NULL,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    INDEX idx_xp_transactions_user_date (user_id, created_at DESC),
    INDEX idx_xp_transactions_source    (source, source_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Tambah kolom XP & level ke tabel users
ALTER TABLE users
    ADD COLUMN total_xp         BIGINT   NOT NULL DEFAULT 0 AFTER updated_at,
    ADD COLUMN current_level    INT      NOT NULL DEFAULT 1 AFTER total_xp,
    ADD COLUMN level_updated_at DATETIME NULL     AFTER current_level;

CREATE INDEX idx_users_total_xp ON users (total_xp DESC);
