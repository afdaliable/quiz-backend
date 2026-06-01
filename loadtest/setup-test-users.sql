-- Buat 100 test user untuk load testing
--
-- CATATAN: /auth/login hanya untuk role admin, dan password TIDAK dicek.
-- Jadi test user harus role='admin', password_hash bisa diisi apa saja.
--
-- Jalankan ke DEV:
--   mysql -h 100.124.237.85 -u afdaliable -p'love4JJI!' dbquizapp < setup-test-users.sql
--
-- Jalankan ke PROD:
--   mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp < setup-test-users.sql

INSERT INTO dbquizapp.users (id, email, display_name, role, created_at, updated_at)
SELECT
  UUID(),
  CONCAT('loadtest', seq, '@test.com'),
  CONCAT('Load Test ', seq),
  'admin',   -- WAJIB admin agar bisa login via /auth/login
  NOW(),
  NOW()
FROM (
  SELECT a.N + b.N * 10 + 1 AS seq
  FROM
    (SELECT 0 AS N UNION SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4
     UNION SELECT 5 UNION SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9) a,
    (SELECT 0 AS N UNION SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4
     UNION SELECT 5 UNION SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9) b
  ORDER BY seq
) seq_table
WHERE NOT EXISTS (
  SELECT 1 FROM dbquizapp.users WHERE email = CONCAT('loadtest', seq, '@test.com')
);

-- Verifikasi
SELECT COUNT(*) AS jumlah_test_users
FROM dbquizapp.users
WHERE email LIKE 'loadtest%@test.com';

-- Hapus setelah selesai testing (jalankan manual):
-- DELETE FROM dbquizapp.users WHERE email LIKE 'loadtest%@test.com';
