-- Pembayaran premium lewat KlikQRIS (QRIS dinamis + webhook).
--
-- signature dari respons /qris/create disimpan di sini dan menjadi dasar
-- validasi webhook: KlikQRIS mengirim signature yang sama, dan webhook yang
-- signaturenya tidak cocok ditolak. Karena validasinya cukup dari DB, instance
-- backend mana pun (PC atau NAS) bisa memproses webhook tanpa kredensial API.
--
-- total_amount = amount + kode unik dari KlikQRIS; itulah yang dibayar user
-- dan yang dicocokkan dengan payload webhook.
CREATE TABLE IF NOT EXISTS dbquizapp.klikqris_transactions (
    order_id        VARCHAR(64)  NOT NULL PRIMARY KEY,
    user_id         VARCHAR(255) NOT NULL,
    plan_id         INT          NOT NULL,
    amount          INT          NOT NULL,
    total_amount    INT          NOT NULL,
    status          ENUM('PENDING','PAID','EXPIRED') NOT NULL DEFAULT 'PENDING',
    signature       VARCHAR(128) NOT NULL,
    qris_url        VARCHAR(512) NULL,
    report_url      VARCHAR(512) NULL,
    expired_at      VARCHAR(32)  NULL,     -- apa adanya dari KlikQRIS (WIB, tanpa zona)
    paid_at         VARCHAR(32)  NULL,
    subscription_id INT          NULL,
    webhook_payload TEXT         NULL,     -- payload terakhir, untuk audit
    created_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    KEY idx_klikqris_user (user_id),
    KEY idx_klikqris_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3 COLLATE=utf8mb3_general_ci;
