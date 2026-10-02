-- Model premium baru: trial 3 hari untuk semua user, lalu langganan bulanan
-- atau tahunan; kode promo dikelola admin.
--
-- Sebelumnya akses premium praktis gratis: pengecek akses mengembalikan true
-- bila paket tidak punya baris premium_quiz_access, dan tabel itu kosong.

-- Harga coret, periode, dan saklar tampil untuk plan.
ALTER TABLE dbquizapp.premium_plans
    ADD COLUMN original_price DOUBLE NULL AFTER price,
    ADD COLUMN period VARCHAR(10) NULL AFTER duration_days,
    ADD COLUMN is_active TINYINT(1) NOT NULL DEFAULT 1 AFTER period,
    ADD COLUMN sort_order INT NOT NULL DEFAULT 0 AFTER is_active;

-- Plan lama disembunyikan (data dan langganan lama tetap utuh).
UPDATE dbquizapp.premium_plans SET is_active = 0;

INSERT INTO dbquizapp.premium_plans
    (name, description, price, original_price, duration_days, period, is_active, sort_order, is_lifetime, features)
VALUES
    ('Bulanan', 'Akses semua paket premium selama 30 hari', 200000, 300000, 30, 'monthly', 1, 1, 0,
     '["Akses semua paket premium", "Simulasi ujian & latihan topik", "Pembahasan lengkap"]'),
    ('Tahunan', 'Akses semua paket premium selama 365 hari, hemat 20%', 1920000, 2400000, 365, 'yearly', 1, 2, 0,
     '["Akses semua paket premium", "Simulasi ujian & latihan topik", "Pembahasan lengkap", "Hemat 20% dibanding bulanan"]');

-- Trial dihitung sejak user pertama kali memakai fitur premium setelah rilis,
-- jadi user lama tidak kehilangan trial.
ALTER TABLE dbquizapp.users ADD COLUMN trial_started_at TIMESTAMP NULL;

-- Kode promo. jenis 'event' berlaku dalam rentang mulai..berakhir untuk semua
-- user; jenis 'akhir_trial' ditawarkan otomatis per user selama durasi_jam
-- setelah trial-nya habis. Waktu disimpan dalam zona DB (WIB).
CREATE TABLE IF NOT EXISTS dbquizapp.promo_codes (
    id                       INT AUTO_INCREMENT PRIMARY KEY,
    kode                     VARCHAR(40)  NOT NULL,
    nama                     VARCHAR(120) NOT NULL,
    jenis                    ENUM('event','akhir_trial') NOT NULL DEFAULT 'event',
    persen                   INT          NOT NULL,
    berlaku_untuk            ENUM('bulanan','tahunan','semua') NOT NULL DEFAULT 'bulanan',
    hanya_pembayaran_pertama TINYINT(1)   NOT NULL DEFAULT 1,
    mulai                    DATETIME     NULL,
    berakhir                 DATETIME     NULL,
    durasi_jam               INT          NULL,
    kuota                    INT          NULL,     -- NULL = tanpa batas
    terpakai                 INT          NOT NULL DEFAULT 0,
    tampilkan_banner         TINYINT(1)   NOT NULL DEFAULT 1,
    teks_banner              VARCHAR(255) NULL,
    aktif                    TINYINT(1)   NOT NULL DEFAULT 1,
    created_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_promo_kode (kode),
    CONSTRAINT ck_promo_persen CHECK (persen BETWEEN 1 AND 100)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

-- Satu kode sekali per user. Dicatat saat pembayaran LUNAS, bukan saat tagihan
-- dibuat, supaya user yang batal bayar tidak kehilangan kodenya.
CREATE TABLE IF NOT EXISTS dbquizapp.promo_redemptions (
    id         INT AUTO_INCREMENT PRIMARY KEY,
    promo_id   INT          NOT NULL,
    user_id    VARCHAR(255) NOT NULL,
    order_id   VARCHAR(64)  NOT NULL,
    potongan   INT          NOT NULL,
    created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_redeem_promo_user (promo_id, user_id),
    KEY idx_redeem_order (order_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

ALTER TABLE dbquizapp.klikqris_transactions
    ADD COLUMN base_amount INT NULL AFTER amount,
    ADD COLUMN discount_amount INT NOT NULL DEFAULT 0 AFTER base_amount,
    ADD COLUMN promo_id INT NULL AFTER discount_amount;

-- Penawaran akhir trial bawaan: 50% bulan pertama, 24 jam setelah trial habis.
INSERT INTO dbquizapp.promo_codes
    (kode, nama, jenis, persen, berlaku_untuk, hanya_pembayaran_pertama, durasi_jam, teks_banner, aktif)
VALUES
    ('TRIAL50', 'Penawaran akhir trial', 'akhir_trial', 50, 'bulanan', 1, 24,
     'Trial kamu sudah habis. Diskon 50% bulan pertama, berakhir dalam', 1);
