-- ============================================================
-- Migration: Sync DEV agar kolom tertentu sesuai PROD (yang lebih baik)
-- Tanggal: 2026-03-11
-- Jalankan di: DEV (localhost)
-- ============================================================

-- 1. harga_paket.harga: ubah dari int → decimal(10,2) seperti PROD
ALTER TABLE `harga_paket`
  MODIFY COLUMN `harga` decimal(10,2) NOT NULL DEFAULT '0.00';

-- 2. soal.sumberfile: perlebar dari varchar(500) → varchar(2000) seperti PROD
ALTER TABLE `soal`
  MODIFY COLUMN `sumberfile` varchar(2000) DEFAULT NULL;

-- 3. Tambah tabel `licenses` (ada di PROD, legacy payment system)
CREATE TABLE IF NOT EXISTS `licenses` (
  `id` int NOT NULL AUTO_INCREMENT,
  `license_code` varchar(255) NOT NULL,
  `user_id` varchar(255) NOT NULL,
  `plan_id` int NOT NULL,
  `status` varchar(50) NOT NULL DEFAULT 'Active',
  `created_at` timestamp NULL DEFAULT CURRENT_TIMESTAMP,
  `expired_at` timestamp NULL DEFAULT NULL,
  `is_lifetime` tinyint(1) DEFAULT '0',
  `product_id` varchar(255) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `license_code` (`license_code`),
  KEY `user_id` (`user_id`),
  KEY `plan_id` (`plan_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci;
