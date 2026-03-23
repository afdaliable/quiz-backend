-- ============================================================
-- Migration: Sync PROD agar sesuai DEV
-- Tanggal: 2026-03-11
-- Jalankan di: PROD (100.87.162.99)
-- ============================================================

-- 1. Tambah tabel `verification` (ada di DEV, belum ada di PROD)
CREATE TABLE IF NOT EXISTS `verification` (
  `id` varchar(255) NOT NULL,
  `identifier` varchar(255) NOT NULL,
  `value` varchar(255) NOT NULL,
  `expiresAt` datetime NOT NULL,
  `createdAt` datetime NOT NULL,
  `updatedAt` datetime NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 2. soal: opt1-5 ubah dari varchar(755) → text (lebih fleksibel untuk soal panjang)
ALTER TABLE `soal`
  MODIFY COLUMN `opt1` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  MODIFY COLUMN `opt2` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  MODIFY COLUMN `opt3` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  MODIFY COLUMN `opt4` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci,
  MODIFY COLUMN `opt5` text CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- 3. soal: modul dan pelajaran perlebar dari varchar(100) → varchar(500)
ALTER TABLE `soal`
  MODIFY COLUMN `modul` varchar(500) DEFAULT NULL,
  MODIFY COLUMN `pelajaran` varchar(500) DEFAULT NULL;

-- 4. jawaban_user: perlebar jawaban dari varchar(255) → varchar(500)
ALTER TABLE `jawaban_user`
  MODIFY COLUMN `jawaban` varchar(500) DEFAULT NULL;

-- 5. users_history: perlebar username dari varchar(255) → varchar(500)
ALTER TABLE `users_history`
  MODIFY COLUMN `username` varchar(500) NOT NULL;

-- ============================================================
-- CATATAN: Tabel berikut TIDAK diubah (biarkan berbeda):
-- - harga_paket.harga: PROD decimal(10,2) > DEV int → PROD lebih baik, biarkan
-- - soal.sumberfile: PROD varchar(2000) > DEV varchar(500) → PROD lebih baik, biarkan
-- - licenses: ada di PROD, tidak ada di DEV → tabel legacy, biarkan di PROD
-- ============================================================
