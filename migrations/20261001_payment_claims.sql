-- Pembayaran QRIS statis (GoPay Merchant) tidak mengirim callback ke server,
-- jadi tidak ada cara otomatis tahu sebuah transaksi lunas. Alurnya: user scan
-- QR dinamis bernominal unik, menekan "saya sudah bayar", akses premium
-- diberikan SEKARANG, pemilik menerima notifikasi Telegram, lalu menyetujui
-- atau menolak setelah mengecek mutasi.
--
-- expires_at adalah jaring pengamannya: klaim yang tidak disetujui dalam 48 jam
-- dicabut otomatis, sehingga lupa mengecek berarti akses mati, bukan gratis
-- selamanya.
CREATE TABLE IF NOT EXISTS dbquizapp.payment_claims (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id         VARCHAR(255) NOT NULL,
    plan_id         INT NOT NULL,
    base_amount     INT NOT NULL,
    -- harga + 3 digit penanda; inilah satu-satunya cara mencocokkan pembayaran
    -- masuk dengan pembelian, karena QRIS statis tidak membawa order id.
    unique_amount   INT NOT NULL,
    status          ENUM('menunggu','diklaim','disetujui','ditolak','kedaluwarsa')
                    NOT NULL DEFAULT 'menunggu',
    subscription_id INT NULL,
    claimed_at      TIMESTAMP NULL,
    expires_at      TIMESTAMP NULL,
    decided_at      TIMESTAMP NULL,
    decided_by      VARCHAR(255) NULL,
    catatan         VARCHAR(255) NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    KEY idx_claims_user (user_id),
    KEY idx_claims_status (status),
    -- nominal unik hanya boleh dipakai satu klaim yang masih berjalan;
    -- dicek di aplikasi karena MariaDB tidak punya unique parsial.
    KEY idx_claims_amount (unique_amount, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3 COLLATE=utf8mb3_general_ci;

-- Siapa yang tidak boleh memakai tombol "saya sudah bayar" lagi, karena klaimnya
-- pernah ditolak (mengaku bayar padahal tidak ada dananya).
CREATE TABLE IF NOT EXISTS dbquizapp.payment_claim_blocks (
    user_id    VARCHAR(255) NOT NULL PRIMARY KEY,
    alasan     VARCHAR(255) NULL,
    blocked_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    blocked_by VARCHAR(255) NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3 COLLATE=utf8mb3_general_ci;
