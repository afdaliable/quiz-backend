# API Endpoint Documentation — Database Queries
> Quiz Backend (Rust / Actix-Web + SQLx + MySQL)
> Generated: 2026-05-30

Dokumen ini memetakan setiap HTTP endpoint ke SQL query yang dieksekusi di database.
Query menggunakan format MySQL dan bisa dijalankan langsung dengan mengganti nilai placeholder
yang ditandai `-- :nama_param`.

---

## Daftar Isi
1. [Auth](#1-auth)
2. [User](#2-user)
3. [Quiz Session](#3-quiz-session)
4. [Soal (Questions)](#4-soal-questions)
5. [Taxonomy](#5-taxonomy)
6. [Premium & Subscription](#6-premium--subscription)
7. [Payment](#7-payment)
8. [License](#8-license)
9. [Simulasi Ujian](#9-simulasi-ujian)
10. [Bookmarks](#10-bookmarks)
11. [Leaderboard](#11-leaderboard)
12. [Search](#12-search)
13. [Analytics](#13-analytics)
14. [Public Profile](#14-public-profile)
15. [Question Feedback & Comments](#15-question-feedback--comments)
16. [Admin — Users](#16-admin--users)
17. [Admin — Soal](#17-admin--soal)
18. [Admin — Packages / Paket Soal](#18-admin--packages--paket-soal)
19. [Admin — Categories](#19-admin--categories)
20. [Admin — Hierarchy & Taxonomy](#20-admin--hierarchy--taxonomy)
21. [Admin — Passages](#21-admin--passages)
22. [Admin — Simulasi Ujian](#22-admin--simulasi-ujian)
23. [Admin — Analytics](#23-admin--analytics)
24. [Admin — Alerts](#24-admin--alerts)
25. [Internal](#25-internal)

---

## 1. Auth

### `POST /auth/login`
Login admin dengan email/password lokal.

```sql
-- Cek user berdasarkan email
SELECT * FROM users WHERE email = 'admin@example.com' AND deleted_at IS NULL;
```

---

### `POST /auth/google/callback`
Google OAuth callback — cari atau buat user, buat session.

```sql
-- Cari user by email
SELECT * FROM users WHERE email = 'user@gmail.com' AND deleted_at IS NULL;

-- Jika user belum ada, buat baru
INSERT INTO users (id, email, display_name, picture_url)
VALUES ('uuid-here', 'user@gmail.com', 'John Doe', 'https://picture.url');

-- Set username jika belum ada
UPDATE users SET username = 'johndoe', updated_at = NOW()
WHERE id = 'uuid-here' AND username IS NULL AND deleted_at IS NULL;

-- Buat session
INSERT INTO sessions (user_id, token, expires_at, ip_address, user_agent)
VALUES ('uuid-here', 'token-here', DATE_ADD(NOW(), INTERVAL 30 DAY), '127.0.0.1', 'Mozilla/...');
```

---

### `POST /auth/validate-session`
Validasi token session.

```sql
SELECT * FROM sessions WHERE token = 'token-here' AND expires_at > NOW();

-- Ambil data user dari session
SELECT * FROM users WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `POST /auth/logout`
Logout — hapus session dari DB dan Redis.

```sql
DELETE FROM sessions WHERE token = 'token-here';
```

---

### `GET /auth/sessions/{user_id}`
Ambil semua session aktif user.

```sql
SELECT * FROM sessions WHERE user_id = 'user-id' AND expires_at > NOW();
```

---

## 2. User

### `GET /user/profile`
Ambil profil user + info subscription aktif.

```sql
-- Profil user
SELECT * FROM users WHERE id = 'user-id' AND deleted_at IS NULL;

-- Subscription aktif
SELECT end_date FROM dbquizapp.user_subscriptions
WHERE user_id = 'user-id'
  AND status = 'active'
  AND (end_date IS NULL OR end_date > NOW())
ORDER BY created_at DESC LIMIT 1;
```

---

### `GET /user/stats`
Statistik belajar user.

```sql
-- Total quiz, skor, benar, salah, pomodoro
SELECT
    COUNT(*) AS total_quizzes,
    COALESCE(SUM(score), 0) AS total_score,
    COALESCE(SUM(correct_answers), 0) AS total_correct,
    COALESCE(SUM(correct_answers + incorrect_answers), 0) AS total_questions,
    COALESCE(SUM(pomodoro_sessions), 0) AS total_pomodoro_sessions,
    COALESCE(SUM(pomodoro_focus_minutes), 0) AS total_pomodoro_minutes
FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE;

-- Kategori favorit
SELECT kategori_soal FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE
GROUP BY kategori_soal
ORDER BY COUNT(*) DESC
LIMIT 1;

-- Streak dates
SELECT DATE(updated_at) AS quiz_date
FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE
GROUP BY DATE(updated_at)
ORDER BY quiz_date DESC;
```

---

### `GET /user/quiz-history`
Riwayat quiz dengan pagination.

```sql
-- Total count
SELECT COUNT(*) FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE;

-- Data history (page 1, 20 per halaman)
SELECT qs.id, qs.paket_soal_id, qs.nama_paket_soal, qs.kategori_soal, qs.session_type, qs.score,
       qs.correct_answers, qs.incorrect_answers,
       COALESCE(
           (SELECT COUNT(*) FROM paket_soal_items psi WHERE psi.paket_soal_id = qs.paket_soal_id),
           JSON_LENGTH(qs.question_ids),
           0
       ) AS total_questions,
       (qs.total_time - COALESCE(qs.time_remaining, 0)) AS duration_seconds,
       qs.updated_at
FROM quiz_sessions qs
WHERE qs.user_id = 'user-id' AND qs.is_completed = TRUE
ORDER BY qs.updated_at DESC
LIMIT 20 OFFSET 0;
```

---

### `PATCH /user/onboarding`
Simpan data onboarding.

```sql
UPDATE users
SET onboarding_completed = TRUE,
    onboarding_goals     = '["cpns","snbt"]',
    exam_timeframe       = '3_months',
    target_exam_date     = '2026-08-01',
    updated_at           = NOW()
WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `POST /users/me/username`
Set/ganti username.

```sql
-- Cek apakah username sudah ada
SELECT COUNT(*) FROM users WHERE username = 'username-baru';

-- Update username
UPDATE users SET username = 'username-baru', updated_at = NOW()
WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `PUT /users/me/privacy`
Update privacy settings.

```sql
UPDATE users
SET profile_public = TRUE,
    privacy_settings = '{"show_score":true}',
    updated_at = NOW()
WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `GET /user/me/xp`
Ambil XP dan level user.

```sql
SELECT total_xp, current_level FROM users WHERE id = 'user-id';
```

---

### `GET /user/preferences`
Ambil preferensi user.

```sql
SELECT preferences FROM users WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `PUT /user/preferences`
Update preferensi user.

```sql
UPDATE users
SET preferences = '{"theme":"dark","language":"id"}',
    updated_at = NOW()
WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

## 3. Quiz Session

### `POST /quiz-session/start`
Mulai quiz session dari paket soal tertentu.

```sql
INSERT INTO quiz_sessions (
    id, user_id, paket_soal_id, kategori_soal, nama_paket_soal,
    session_type, question_ids,
    total_time, current_question, is_completed, score,
    correct_answers, incorrect_answers, created_at, updated_at
)
VALUES (
    'session-uuid', 'user-id', 42, 'CPNS', 'Paket CPNS 2024',
    'standard', NULL,
    3600, 0, FALSE, 0, 0, 0, NOW(), NOW()
);
```

---

### `POST /quiz-session/start-random`
Mulai quiz random dari soal yang bisa diakses user.

```sql
-- Ambil soal yang bisa diakses user (dengan kategori)
SELECT DISTINCT s.id FROM soal s
INNER JOIN paket_soal_items psi ON psi.soal_id = s.id
INNER JOIN paket_soal p ON p.id = psi.paket_soal_id
INNER JOIN kategori_soal k ON k.id = p.kategori_id
LEFT JOIN premium_quiz_access pqa ON pqa.paket_soal_id = p.id
WHERE (
    p.is_premium = FALSE
    OR EXISTS (
        SELECT 1 FROM user_subscriptions us
        WHERE us.user_id = 'user-id'
          AND us.status = 'active'
          AND (us.end_date IS NULL OR us.end_date > NOW())
          AND us.plan_id >= pqa.min_plan_id
    )
)
AND k.nama_kategori = 'CPNS';

-- Fetch soal yang terpilih (random 20)
SELECT id, soal, question_type, opt1, opt2, opt3, opt4, opt5, solution, modul, pelajaran, tag
FROM soal WHERE id IN (101, 205, 312); -- id hasil random dari query atas

-- Buat session random
INSERT INTO quiz_sessions (
    id, user_id, paket_soal_id, kategori_soal, nama_paket_soal,
    session_type, question_ids,
    total_time, current_question, is_completed, score,
    correct_answers, incorrect_answers, created_at, updated_at
)
VALUES (
    'session-uuid', 'user-id', NULL, 'CPNS', 'Latihan Random',
    'random', '[101,205,312]',
    1200, 0, FALSE, 0, 0, 0, NOW(), NOW()
);
```

---

### `POST /quiz-session/check`
Cek apakah session aktif sudah ada untuk paket+kategori ini.

```sql
SELECT * FROM quiz_sessions
WHERE user_id = 'user-id'
  AND paket_soal_id = 42
  AND kategori_soal = 'CPNS'
  AND nama_paket_soal = 'Paket CPNS 2024'
  AND is_completed = FALSE
ORDER BY created_at DESC
LIMIT 1;
```

---

### `GET /quiz-session/active`
Ambil semua session yang sedang aktif (belum selesai).

```sql
SELECT * FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = FALSE
ORDER BY created_at DESC;
```

---

### `GET /quiz-session/completed`
Ambil session yang sudah selesai.

```sql
SELECT * FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE
ORDER BY created_at DESC
LIMIT 10;
```

---

### `GET /quiz-session/{id}`
Ambil detail satu session.

```sql
SELECT * FROM quiz_sessions WHERE id = 'session-uuid' AND user_id = 'user-id';
```

---

### `PUT /quiz-session/{id}/save`
Simpan progres quiz (jawaban, posisi soal, waktu tersisa).

```sql
UPDATE quiz_sessions
SET updated_at = NOW(),
    current_question = 5,
    answers = '{"101":"A","205":"C"}',
    marked_questions = '[205]',
    time_remaining = 2800
WHERE id = 'session-uuid' AND user_id = 'user-id' AND is_completed = FALSE;
```

---

### `PUT /quiz-session/{id}/complete`
Selesaikan quiz dan hitung skor + award XP.

```sql
-- 1. Hitung skor
SELECT id, correct_answer FROM soal WHERE id IN (101, 205, 312);

-- 2. Selesaikan session
UPDATE quiz_sessions
SET answers = '{"101":"A","205":"C","312":"B"}',
    time_remaining = 0,
    is_completed = TRUE,
    score = 85,
    correct_answers = 17,
    incorrect_answers = 3,
    pomodoro_enabled = FALSE,
    pomodoro_sessions = 0,
    pomodoro_focus_minutes = 0,
    pomodoro_questions_answered = 0,
    updated_at = NOW()
WHERE id = 'session-uuid' AND user_id = 'user-id' AND is_completed = FALSE;

-- 3. Award XP: cek XP hari ini
SELECT
    u.total_xp,
    CAST(COALESCE(SUM(CASE
        WHEN DATE(CONVERT_TZ(t.created_at, '+00:00', '+07:00')) = '2026-05-30' THEN t.amount
        ELSE 0
    END), 0) AS SIGNED) AS today_xp,
    u.current_level
FROM users u
LEFT JOIN xp_transactions t ON t.user_id = u.id
WHERE u.id = 'user-id'
GROUP BY u.id, u.total_xp, u.current_level;

-- 4. Simpan XP transaction
INSERT INTO xp_transactions (id, user_id, amount, source, source_id, description)
VALUES ('xp-uuid', 'user-id', 50, 'quiz_complete', 'session-uuid', 'Quiz: Paket CPNS 2024 — Skor 85');

-- 5. Update total XP user
UPDATE users
SET total_xp = 1050,
    current_level = 5,
    level_updated_at = CASE WHEN current_level != 5 THEN NOW() ELSE level_updated_at END
WHERE id = 'user-id';

-- 6. Simpan attempt simulasi (jika ini session simulasi)
UPDATE simulasi_user_attempts
SET score = 85, is_passed = TRUE, completed_at = CURRENT_TIMESTAMP
WHERE quiz_session_id = 'session-uuid';
```

---

### `DELETE /quiz-session/{id}`
Hapus session.

```sql
DELETE FROM quiz_sessions WHERE id = 'session-uuid' AND user_id = 'user-id';
```

---

## 4. Soal (Questions)

### `GET /soal/{id}`
Ambil soal by ID.

```sql
SELECT * FROM soal WHERE id = 101;
```

---

### `GET /soal`
List soal dengan filter taxonomy + pagination.

```sql
-- Contoh: filter by track + category + difficulty=medium + status=active, page 1
SELECT * FROM soal
WHERE track_id = 1
  AND category_id = 3
  AND difficulty_est = 'medium'
  AND status = 'active'
ORDER BY id
LIMIT 20 OFFSET 0;
```

---

### `GET /paket-soal-response/{nama_kategori}/{nama_paket_soal}`
Ambil soal lengkap dalam paket (dengan cek akses premium).

```sql
-- Cek akses premium user ke paket
SELECT COUNT(*)
FROM dbquizapp.user_subscriptions us
JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
WHERE us.user_id = 'user-id'
  AND us.status = 'active'
  AND (us.end_date IS NULL OR us.end_date > NOW())
  AND pp.id >= 2; -- min_plan_id dari premium_quiz_access

-- Ambil soal dalam paket berdasarkan kategori + nama paket
SELECT s.correct_answer
FROM kategori_soal ks
JOIN paket_soal ps ON ks.id = ps.kategori_id
JOIN paket_soal_items psi ON psi.paket_soal_id = ps.id
JOIN soal s ON psi.soal_id = s.id
WHERE ks.nama_kategori = 'CPNS'
  AND ps.nama_paket_soal = 'Paket CPNS 2024'
ORDER BY psi.id;
```

---

### `GET /listpaketsoal`
List semua paket soal (cached Redis).

```sql
SELECT * FROM paket_soal ORDER BY id;
```

---

### `GET /check-quiz-access/{paket_soal_id}`
Cek apakah user bisa akses quiz package tertentu.

```sql
SELECT COUNT(*)
FROM dbquizapp.user_subscriptions us
JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
WHERE us.user_id = 'user-id'
  AND us.status = 'active'
  AND (us.end_date IS NULL OR us.end_date > NOW())
  AND pp.id >= (
      SELECT min_plan_id FROM premium_quiz_access WHERE paket_soal_id = 42
  );
```

---

### `GET /soal/search`
Cari soal dengan full-text + filter.

```sql
SELECT * FROM soal
WHERE (soal LIKE '%integral%' OR opt1 LIKE '%integral%' OR solution LIKE '%integral%')
  AND modul = 'Matematika'
  AND status = 'active'
ORDER BY id
LIMIT 20 OFFSET 0;
```

---

### `GET /soal/search/filters`
Ambil daftar modul dan pelajaran yang tersedia.

```sql
SELECT DISTINCT modul FROM soal WHERE modul IS NOT NULL ORDER BY modul;
SELECT DISTINCT pelajaran FROM soal WHERE pelajaran IS NOT NULL ORDER BY pelajaran;
```

---

## 5. Taxonomy

### `GET /tracks`
Semua track ujian.

```sql
SELECT t.id, t.slug, t.name, t.icon, t.status, t.sort_order,
       COUNT(s.id) AS question_count
FROM exam_tracks t
LEFT JOIN soal s ON s.track_id = t.id
GROUP BY t.id, t.slug, t.name, t.icon, t.status, t.sort_order
ORDER BY t.sort_order;
```

---

### `GET /tracks/{slug}/categories`
Kategori untuk track tertentu.

```sql
SELECT c.id, c.track_id, c.slug, c.name, c.sort_order,
       COUNT(s.id) AS question_count
FROM categories c
JOIN exam_tracks t ON t.id = c.track_id
LEFT JOIN soal s ON s.category_id = c.id
WHERE t.slug = 'cpns'
GROUP BY c.id, c.track_id, c.slug, c.name, c.sort_order
ORDER BY c.sort_order;
```

---

### `GET /categories/{slug}/subcategories`
Subkategori untuk category slug.

```sql
SELECT sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order,
       COUNT(s.id) AS question_count
FROM subcategories sc
JOIN categories c ON c.id = sc.category_id
LEFT JOIN soal s ON s.subcategory_id = sc.id
WHERE c.slug = 'twk'
GROUP BY sc.id, sc.category_id, sc.slug, sc.name, sc.sort_order
ORDER BY sc.sort_order;
```

---

### `GET /subcategories/{slug}/topics`
Topics untuk subcategory slug.

```sql
SELECT tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order,
       COUNT(s.id) AS question_count
FROM topics tp
JOIN subcategories sc ON sc.id = tp.subcategory_id
LEFT JOIN soal s ON s.topic_id = tp.id
WHERE sc.slug = 'nasionalisme'
GROUP BY tp.id, tp.subcategory_id, tp.slug, tp.name, tp.sort_order
ORDER BY tp.sort_order;
```

---

### `GET /topics/{slug}/tags`
Tags untuk topic slug.

```sql
SELECT tg.id, tg.slug, tg.label
FROM tags tg
JOIN question_tags qt ON qt.tag_id = tg.id
JOIN soal s ON s.id = qt.question_id
JOIN topics tp ON tp.id = s.topic_id
WHERE tp.slug = 'pancasila'
GROUP BY tg.id, tg.slug, tg.label
ORDER BY tg.label;
```

---

### `GET /tags`
Search tags.

```sql
SELECT id, slug, label FROM tags
WHERE slug LIKE '%pancasila%' OR label LIKE '%pancasila%'
ORDER BY label
LIMIT 30;
```

---

### `GET /taxonomy/tree`
Full taxonomy tree.

```sql
-- Semua track
SELECT id, slug, name, icon, status, sort_order, 0 AS question_count FROM exam_tracks ORDER BY sort_order;

-- Semua categories
SELECT id, track_id, slug, name, sort_order, 0 AS question_count FROM categories ORDER BY sort_order;

-- Semua subcategories
SELECT id, category_id, slug, name, sort_order, 0 AS question_count FROM subcategories ORDER BY sort_order;

-- Semua topics
SELECT id, subcategory_id, slug, name, sort_order, 0 AS question_count FROM topics ORDER BY sort_order;
```

---

## 6. Premium & Subscription

### `GET /premium/plans`
Semua plan premium.

```sql
SELECT * FROM dbquizapp.premium_plans ORDER BY price ASC;
```

---

### `GET /premium/check-status`
Cek apakah user punya subscription aktif.

```sql
SELECT COUNT(*) FROM dbquizapp.user_subscriptions
WHERE user_id = 'user-id'
  AND status = 'active'
  AND (end_date IS NULL OR end_date > NOW());
```

---

### `GET /premium/subscriptions`
Daftar subscription user.

```sql
SELECT
    us.id, us.user_id, us.plan_id,
    pp.name as plan_name,
    us.status, us.start_date, us.end_date,
    pp.is_lifetime
FROM dbquizapp.user_subscriptions us
JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
WHERE us.user_id = 'user-id'
ORDER BY us.created_at DESC;
```

---

### `GET /premium/subscriptions/active`
Subscription aktif user.

```sql
SELECT
    us.id, us.user_id, us.plan_id,
    pp.name as plan_name,
    us.start_date, us.end_date,
    us.status, pp.is_lifetime
FROM dbquizapp.user_subscriptions us
JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
WHERE us.user_id = 'user-id'
  AND us.status = 'active'
  AND (us.end_date IS NULL OR us.end_date > NOW())
ORDER BY us.created_at DESC
LIMIT 1;
```

---

### `POST /premium/subscriptions`
Buat subscription baru (biasanya dipanggil setelah payment sukses).

```sql
-- Ambil detail plan
SELECT duration_days, is_lifetime FROM dbquizapp.premium_plans WHERE id = 2;

-- Buat subscription (end_date = NULL untuk lifetime, atau NOW() + duration_days)
INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status)
VALUES ('user-id', 2, NOW(), DATE_ADD(NOW(), INTERVAL 30 DAY), 'active');
```

---

### `POST /premium/subscriptions/{id}/cancel`
Batalkan subscription.

```sql
UPDATE dbquizapp.user_subscriptions SET status = 'cancelled' WHERE id = 5;
```

---

### `PUT /premium/subscriptions/{id}`
Update subscription (perpanjang / ubah status).

```sql
UPDATE dbquizapp.user_subscriptions
SET status = 'active', end_date = '2027-05-30'
WHERE id = 5;
```

---

## 7. Payment

### `GET /payment/transactions`
Daftar transaksi payment user.

```sql
SELECT
    pt.id, pt.user_id, pt.plan_id,
    pp.name as plan_name,
    pt.amount, pt.transaction_id, pt.payment_link,
    pt.status, pt.payment_method, pt.created_at
FROM dbquizapp.payment_transactions pt
JOIN dbquizapp.premium_plans pp ON pt.plan_id = pp.id
WHERE pt.user_id = 'user-id'
ORDER BY pt.created_at DESC;
```

---

### `GET /payment/transactions/{id}`
Detail satu transaksi.

```sql
SELECT
    pt.id, pt.user_id, pt.plan_id,
    pp.name as plan_name,
    pt.amount, pt.transaction_id, pt.payment_link,
    pt.status, pt.payment_method, pt.created_at
FROM dbquizapp.payment_transactions pt
JOIN dbquizapp.premium_plans pp ON pt.plan_id = pp.id
WHERE pt.id = 10;
```

---

### `POST /payment/create`
Buat transaksi payment baru (Mayar).

```sql
INSERT INTO dbquizapp.payment_transactions
(user_id, plan_id, amount, transaction_id, payment_link, status, payment_details)
VALUES ('user-id', 2, 99000, 'TXN-abc123', 'https://mayar.id/pay/abc', 'pending', '{"mayar_id":"xxx"}');
```

---

### `POST /payment/webhook`
Webhook Mayar — update status transaksi + buat subscription jika sukses.

```sql
-- Update status transaksi
UPDATE dbquizapp.payment_transactions
SET status = 'paid', payment_method = 'bank_transfer', webhook_data = '{"raw":"data"}'
WHERE transaction_id = 'TXN-abc123';

-- Buat subscription jika status = paid
SELECT duration_days, is_lifetime FROM dbquizapp.premium_plans WHERE id = 2;

INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status)
VALUES ('user-id', 2, NOW(), DATE_ADD(NOW(), INTERVAL 30 DAY), 'active');
```

---

### `GET /payment/check/{transaction_id}`
Cek status transaksi di DB.

```sql
SELECT * FROM dbquizapp.payment_transactions WHERE transaction_id = 'TXN-abc123';
```

---

### `GET /payment/check-phone/{user_id}`
Cek apakah user punya nomor HP (wajib sebelum bayar).

```sql
SELECT * FROM dbquizapp.users WHERE phone_number = 'user-id'; -- sebenarnya WHERE id = ?
```

---

## 8. License

### `GET /license/user-licenses`
Daftar lisensi milik user.

```sql
SELECT * FROM dbquizapp.license_codes
WHERE user_id = 'user-id'
ORDER BY created_at DESC;
```

---

### `GET /license/payment-link/{plan_id}`
Generate link payment Mayar untuk plan.

```sql
SELECT * FROM dbquizapp.premium_plans WHERE id = 2;
```

---

### `POST /license-public/verify`
Verifikasi license code Mayar → buat user & subscription.

```sql
-- Cek lisensi
SELECT * FROM dbquizapp.license_codes WHERE license_code = 'LIC-xxx';

-- Jika user belum ada, buat dari data customer di license
INSERT INTO users (id, email, display_name) VALUES ('uuid', 'email@ex.com', 'John');

-- Update license dengan user_id
UPDATE dbquizapp.license_codes SET user_id = 'uuid' WHERE license_code = 'LIC-xxx';

-- Update status lisensi
UPDATE dbquizapp.license_codes SET status = 'active' WHERE license_code = 'LIC-xxx';

-- Buat subscription
INSERT INTO dbquizapp.user_subscriptions (user_id, plan_id, start_date, end_date, status)
VALUES ('uuid', 2, NOW(), DATE_ADD(NOW(), INTERVAL 365 DAY), 'active');
```

---

## 9. Simulasi Ujian

### `GET /simulasi-ujian`
Daftar simulasi ujian untuk user.

```sql
SELECT
    s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id,
    ps.nama_paket_soal AS paket_soal_nama,
    s.generation_mode, s.generation_config,
    s.duration_minutes, s.total_questions, s.passing_score,
    s.is_premium, s.max_attempts, s.is_active,
    s.created_at, s.updated_at,
    COALESCE(ua.user_attempts, 0) AS user_attempts,
    ua.best_score,
    CASE WHEN s.max_attempts = 0 OR COALESCE(ua.user_attempts, 0) < s.max_attempts
         THEN TRUE ELSE FALSE END AS can_attempt
FROM exam_simulations s
LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
LEFT JOIN (
    SELECT simulasi_id, COUNT(*) AS user_attempts, MAX(score) AS best_score
    FROM simulasi_user_attempts
    WHERE user_id = 'user-id'
    GROUP BY simulasi_id
) ua ON ua.simulasi_id = s.id
WHERE s.is_active = TRUE
ORDER BY s.created_at DESC;
```

---

### `GET /simulasi-ujian/{id}`
Detail simulasi + attempt user.

```sql
SELECT s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id,
       ps.nama_paket_soal AS paket_soal_nama,
       s.generation_mode, s.generation_config,
       s.duration_minutes, s.total_questions, s.passing_score,
       s.is_premium, s.max_attempts, s.is_active,
       s.created_at, s.updated_at
FROM exam_simulations s
LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
WHERE s.id = 1;
```

---

### `POST /simulasi-ujian/{id}/start`
Mulai simulasi — hitung sisa attempt, buat quiz session + attempt record.

```sql
-- Cek jumlah attempt user
SELECT COUNT(*) FROM simulasi_user_attempts WHERE simulasi_id = 1 AND user_id = 'user-id';

-- Ambil soal dari paket_soal simulasi
SELECT soal_id FROM paket_soal_items WHERE paket_soal_id = 5;

-- Buat quiz session
INSERT INTO quiz_sessions (id, user_id, paket_soal_id, kategori_soal, nama_paket_soal,
    session_type, question_ids, total_time, current_question, is_completed, score,
    correct_answers, incorrect_answers, created_at, updated_at)
VALUES ('session-uuid', 'user-id', 5, 'Simulasi', 'Try Out CPNS 2024',
    'standard', NULL, 5400, 0, FALSE, 0, 0, 0, NOW(), NOW());

-- Catat attempt
INSERT INTO simulasi_user_attempts (simulasi_id, user_id, quiz_session_id, attempt_number)
VALUES (1, 'user-id', 'session-uuid', 1);
```

---

### `GET /simulasi-ujian/{id}/my-attempts`
Riwayat attempt user untuk simulasi ini.

```sql
SELECT id, simulasi_id, user_id, quiz_session_id, attempt_number,
       score, is_passed, completed_at, created_at,
       NULL AS user_email, NULL AS user_display_name
FROM simulasi_user_attempts
WHERE simulasi_id = 1 AND user_id = 'user-id'
ORDER BY attempt_number DESC;
```

---

## 10. Bookmarks

### `POST /bookmarks/{question_id}`
Bookmark soal.

```sql
INSERT INTO bookmarked_questions (user_id, question_id)
VALUES ('user-id', 101);
```

---

### `DELETE /bookmarks/{question_id}`
Hapus bookmark.

```sql
DELETE FROM bookmarked_questions WHERE user_id = 'user-id' AND question_id = 101;
```

---

### `DELETE /bookmarks/bulk`
Hapus banyak bookmark sekaligus.

```sql
DELETE FROM bookmarked_questions
WHERE user_id = 'user-id' AND question_id IN (101, 205, 312);
```

---

### `GET /bookmarks`
Daftar soal yang di-bookmark user (paginated).

```sql
SELECT bq.*, s.soal, s.opt1, s.opt2, s.opt3, s.opt4, s.correct_answer
FROM bookmarked_questions bq
JOIN soal s ON s.id = bq.question_id
WHERE bq.user_id = 'user-id'
ORDER BY bq.created_at DESC
LIMIT 20 OFFSET 0;
```

---

## 11. Leaderboard

### `GET /leaderboard/xp`
Top 100 XP leaderboard + ranking user saat ini.

```sql
SELECT u.id, u.display_name, u.picture_url, u.total_xp, u.current_level,
       RANK() OVER (ORDER BY u.total_xp DESC) AS rank
FROM users u
WHERE u.profile_public = TRUE AND u.deleted_at IS NULL
ORDER BY u.total_xp DESC
LIMIT 100;
```

---

## 12. Search

### `GET /soal/search`
Cari soal by keyword.

```sql
SELECT * FROM soal
WHERE soal LIKE '%integral%'
  AND status = 'active'
ORDER BY id
LIMIT 20 OFFSET 0;
```

---

### `GET /soal/search/filters`
Ambil filter yang tersedia.

```sql
SELECT DISTINCT modul FROM soal WHERE modul IS NOT NULL ORDER BY modul;
SELECT DISTINCT pelajaran FROM soal WHERE pelajaran IS NOT NULL ORDER BY pelajaran;
```

---

## 13. Analytics

### `GET /analytics/score-history`
Riwayat skor user untuk chart.

```sql
-- Data points (semua quiz yang selesai, diurutkan ASC)
SELECT
    qs.id AS session_id,
    qs.paket_soal_id,
    qs.nama_paket_soal AS package_name,
    qs.kategori_soal   AS category,
    qs.score,
    qs.correct_answers  AS correct,
    qs.incorrect_answers AS incorrect,
    COALESCE(
        (SELECT COUNT(*) FROM paket_soal_items psi WHERE psi.paket_soal_id = qs.paket_soal_id),
        JSON_LENGTH(qs.question_ids),
        0
    ) AS total_questions,
    (qs.total_time - COALESCE(qs.time_remaining, 0)) AS duration_seconds,
    qs.updated_at AS completed_at
FROM quiz_sessions qs
WHERE user_id = 'user-id' AND is_completed = TRUE
ORDER BY qs.updated_at ASC;

-- Summary statistik
SELECT
    COALESCE(AVG(score), 0)  AS average,
    COALESCE(MAX(score), 0)  AS highest,
    COALESCE(MIN(score), 0)  AS lowest,
    COUNT(*)                  AS total_attempts,
    COALESCE(AVG(CASE WHEN updated_at > DATE_SUB(NOW(), INTERVAL 15 DAY) THEN score END), 0)
    - COALESCE(AVG(CASE WHEN updated_at BETWEEN DATE_SUB(NOW(), INTERVAL 30 DAY) AND DATE_SUB(NOW(), INTERVAL 15 DAY) THEN score END), 0)
    AS trend
FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE;
```

---

## 14. Public Profile

### `GET /users/profile/{username}`
Ambil profil publik user.

```sql
-- Ambil user by username
SELECT * FROM users WHERE username = 'johndoe' AND deleted_at IS NULL;

-- Statistik publik (jika profile_public = TRUE)
SELECT
    COUNT(*) AS total_quizzes,
    COALESCE(AVG(score), 0) AS avg_score,
    COALESCE(MAX(score), 0) AS best_score
FROM quiz_sessions
WHERE user_id = 'user-id' AND is_completed = TRUE;

SELECT total_xp, current_level FROM users WHERE id = 'user-id';
```

---

### `GET /users/username/check`
Cek ketersediaan username.

```sql
SELECT COUNT(*) FROM users WHERE username = 'johndoe';
```

---

## 15. Question Feedback & Comments

### `POST /questions/{id}/rate`
Rate soal (helpful/confusing) — upsert.

```sql
INSERT INTO question_ratings (question_id, user_id, rating)
VALUES (101, 'user-id', 'helpful')
ON DUPLICATE KEY UPDATE rating = VALUES(rating), updated_at = NOW();

-- Hapus rating (jika user un-rate)
DELETE FROM question_ratings WHERE question_id = 101 AND user_id = 'user-id';
```

---

### `GET /questions/{id}/ratings`
Ambil summary rating soal.

```sql
SELECT COUNT(*) FROM question_ratings WHERE question_id = 101 AND rating = 'helpful';
SELECT COUNT(*) FROM question_ratings WHERE question_id = 101 AND rating = 'confusing';
SELECT rating FROM question_ratings WHERE question_id = 101 AND user_id = 'user-id';
```

---

### `POST /questions/{id}/report`
Laporkan soal.

```sql
-- Cek apakah sudah pernah report (pending)
SELECT id FROM question_reports
WHERE question_id = 101 AND user_id = 'user-id' AND status = 'pending';

-- Jika ada, update
UPDATE question_reports
SET reason = 'wrong_answer', detail = 'Jawaban harusnya B bukan A', updated_at = NOW()
WHERE id = 55;

-- Jika belum ada, insert
INSERT INTO question_reports (id, question_id, user_id, reason, detail)
VALUES ('report-uuid', 101, 'user-id', 'wrong_answer', 'Jawaban harusnya B bukan A');
```

---

### `GET /questions/{question_id}/comments`
List komentar pada soal.

```sql
SELECT * FROM question_comments
WHERE question_id = 101 AND parent_id IS NULL
ORDER BY created_at DESC
LIMIT 20 OFFSET 0;
```

---

### `POST /questions/{question_id}/comments`
Buat komentar atau reply.

```sql
INSERT INTO question_comments (id, question_id, user_id, parent_id, content)
VALUES ('comment-uuid', 101, 'user-id', NULL, 'Soal ini bagus sekali!');
```

---

### `POST /comments/{comment_id}/upvote`
Toggle upvote komentar.

```sql
INSERT INTO comment_upvotes (comment_id, user_id) VALUES ('comment-uuid', 'user-id')
ON DUPLICATE KEY UPDATE created_at = NOW();

-- Atau hapus jika sudah ada
DELETE FROM comment_upvotes WHERE comment_id = 'comment-uuid' AND user_id = 'user-id';
```

---

## 16. Admin — Users

### `GET /admin/users`
Daftar semua user dengan filter + pagination.

```sql
SELECT u.*, us.status AS subscription_status, us.end_date AS subscription_end
FROM users u
LEFT JOIN user_subscriptions us ON us.user_id = u.id
    AND us.status = 'active' AND (us.end_date IS NULL OR us.end_date > NOW())
WHERE u.deleted_at IS NULL
ORDER BY u.created_at DESC
LIMIT 20 OFFSET 0;
```

---

### `GET /admin/users/stats`
Statistik user.

```sql
SELECT COUNT(*) AS total_users FROM users WHERE deleted_at IS NULL;
SELECT COUNT(*) AS new_users_today FROM users
WHERE DATE(created_at) = CURDATE() AND deleted_at IS NULL;
SELECT COUNT(DISTINCT us.user_id) AS premium_users
FROM user_subscriptions us
WHERE us.status = 'active' AND (us.end_date IS NULL OR us.end_date > NOW());
```

---

### `GET /admin/users/{id}`
Detail user by ID.

```sql
SELECT * FROM users WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `PUT /admin/users/{id}`
Update data user.

```sql
UPDATE users
SET display_name = 'New Name', phone_number = '08123456789', updated_at = NOW()
WHERE id = 'user-id' AND deleted_at IS NULL;
```

---

### `DELETE /admin/users/{id}`
Soft delete user.

```sql
UPDATE users SET deleted_at = NOW() WHERE id = 'user-id';
-- Hapus semua session
DELETE FROM sessions WHERE user_id = 'user-id';
```

---

### `GET /admin/users/{id}/subscriptions`
Subscription history user.

```sql
SELECT
    us.id, us.user_id, us.plan_id,
    pp.name as plan_name,
    us.status, us.start_date, us.end_date,
    pp.is_lifetime
FROM dbquizapp.user_subscriptions us
JOIN dbquizapp.premium_plans pp ON us.plan_id = pp.id
WHERE us.user_id = 'user-id'
ORDER BY us.created_at DESC;
```

---

## 17. Admin — Soal

### `GET /admin/soal/search`
Cari soal di admin dengan filter lengkap.

```sql
SELECT s.*, t.name AS track_name, c.name AS category_name,
       sc.name AS subcategory_name, tp.name AS topic_name
FROM soal s
LEFT JOIN exam_tracks t ON t.id = s.track_id
LEFT JOIN categories c ON c.id = s.category_id
LEFT JOIN subcategories sc ON sc.id = s.subcategory_id
LEFT JOIN topics tp ON tp.id = s.topic_id
WHERE s.status = 'active'
  AND s.track_id = 1
  AND s.difficulty_est = 'medium'
  AND (s.soal LIKE '%integral%')
ORDER BY s.id DESC
LIMIT 20 OFFSET 0;
```

---

### `GET /admin/soal/coverage-stats`
Statistik coverage soal (berapa soal per topik/difficulty).

```sql
SELECT t.id as topic_id, t.name as topic_name,
    SUM(CASE WHEN s.difficulty_est = 'easy' THEN 1 ELSE 0 END) as easy,
    SUM(CASE WHEN s.difficulty_est = 'medium' THEN 1 ELSE 0 END) as medium,
    SUM(CASE WHEN s.difficulty_est = 'hard' THEN 1 ELSE 0 END) as hard
FROM topics t
LEFT JOIN soal s ON s.topic_id = t.id
GROUP BY t.id, t.name
ORDER BY (easy + medium + hard) ASC
LIMIT 30;
```

---

### `POST /admin/soal`
Buat soal baru.

```sql
INSERT INTO dbquizapp.soal (
    passage_id, soal, question_type, opt1, opt2, opt3, opt4, opt5,
    correct_answer, solution, sumberfile, modul, pelajaran, tag,
    track_id, category_id, subcategory_id, topic_id,
    difficulty_est, difficulty_calc, bloom_level, format, source, status
)
VALUES (
    NULL,
    'Berapakah hasil dari 2 + 2?',
    'multiple_choice',
    '3', '4', '5', '6', NULL,
    'B', 'Penjumlahan sederhana: 2 + 2 = 4',
    NULL, 'Matematika', 'Aritmatika', NULL,
    1, 3, 5, 12,
    'easy', NULL, 'remember', 'text', 'internal', 'active'
);

SELECT * FROM soal WHERE id = LAST_INSERT_ID();
```

---

### `GET /admin/soal/{id}`
Detail soal by ID (admin).

```sql
SELECT * FROM soal WHERE id = 101;

-- Ambil tags soal
SELECT tg.id, tg.slug, tg.label
FROM tags tg
JOIN question_tags qt ON qt.tag_id = tg.id
WHERE qt.question_id = 101
ORDER BY tg.label;
```

---

### `PUT /admin/soal/{id}`
Update soal.

```sql
UPDATE soal
SET soal = 'Teks soal baru',
    correct_answer = 'C',
    solution = 'Penjelasan baru',
    difficulty_est = 'hard',
    updated_at = NOW()
WHERE id = 101;

-- Sync tags (hapus lama, insert baru)
DELETE FROM question_tags WHERE question_id = 101;
INSERT IGNORE INTO question_tags (question_id, tag_id) VALUES (101, 5), (101, 12);
```

---

### `DELETE /admin/soal/{id}`
Hapus soal.

```sql
DELETE FROM soal WHERE id = 101;
```

---

### `GET /admin/soal/{id}/packages`
Semua paket yang memuat soal ini.

```sql
SELECT p.id, p.nama_paket_soal, p.kategori_id
FROM paket_soal_items psi
JOIN paket_soal p ON p.id = psi.paket_soal_id
WHERE psi.soal_id = 101;
```

---

### `PATCH /admin/questions/bulk`
Bulk update taxonomy/status soal.

```sql
UPDATE soal
SET track_id = 1, category_id = 3, subcategory_id = 5,
    topic_id = 12, difficulty_est = 'medium', status = 'active'
WHERE id IN (101, 205, 312, 400);
```

---

## 18. Admin — Packages / Paket Soal

### `GET /admin/packages`
List paket soal (admin, dengan pagination + filter).

```sql
SELECT p.*, k.nama_kategori,
       COUNT(psi.id) AS total_questions
FROM paket_soal p
LEFT JOIN kategori_soal k ON k.id = p.kategori_id
LEFT JOIN paket_soal_items psi ON psi.paket_soal_id = p.id
GROUP BY p.id
ORDER BY p.id DESC
LIMIT 20 OFFSET 0;
```

---

### `POST /admin/packages`
Buat paket soal baru.

```sql
INSERT INTO paket_soal (nama_paket_soal, kategori_id, is_premium, deskripsi)
VALUES ('Paket CPNS 2025', 1, FALSE, 'Paket latihan CPNS tahun 2025');
```

---

### `GET /admin/packages/{id}/questions`
Soal dalam paket (admin).

```sql
SELECT s.id, s.soal, s.difficulty_est, s.status, psi.id AS item_id
FROM paket_soal_items psi
JOIN soal s ON s.id = psi.soal_id
WHERE psi.paket_soal_id = 5
ORDER BY psi.id;
```

---

### `POST /admin/packages/{id}/questions`
Tambah soal ke paket.

```sql
INSERT INTO paket_soal_items (paket_soal_id, soal_id)
VALUES (5, 101), (5, 205), (5, 312);
```

---

### `DELETE /admin/packages/{id}/questions/{question_id}`
Hapus soal dari paket.

```sql
DELETE FROM paket_soal_items
WHERE paket_soal_id = 5 AND soal_id = 101;
```

---

### `DELETE /admin/packages/{id}`
Hapus paket soal (beserta items).

```sql
DELETE FROM paket_soal_items WHERE paket_soal_id = 5;
DELETE FROM paket_soal WHERE id = 5;
```

---

## 19. Admin — Categories

### `GET /admin/categories`
Semua kategori soal dengan jumlah soal.

```sql
SELECT k.*, COUNT(p.id) AS total_paket
FROM kategori_soal k
LEFT JOIN paket_soal p ON p.kategori_id = k.id
GROUP BY k.id
ORDER BY k.id;
```

---

### `POST /admin/categories`
Buat kategori soal baru.

```sql
INSERT INTO kategori_soal (nama_kategori, deskripsi)
VALUES ('PPPK', 'Latihan soal PPPK');
```

---

### `PUT /admin/categories/{id}`
Update kategori.

```sql
UPDATE kategori_soal
SET nama_kategori = 'CPNS 2025', deskripsi = 'Updated desc'
WHERE id = 3;
```

---

### `DELETE /admin/categories/{id}`
Hapus kategori.

```sql
DELETE FROM kategori_soal WHERE id = 3;
```

---

## 20. Admin — Hierarchy & Taxonomy

### `GET /admin/hierarchy/tracks`
List semua tracks (admin).

```sql
SELECT t.id, t.slug, t.name, t.icon, t.status, t.sort_order,
       (SELECT COUNT(DISTINCT s.id) FROM soal s WHERE s.track_id = t.id) AS question_count
FROM exam_tracks t
ORDER BY t.sort_order;
```

---

### `POST /admin/hierarchy/tracks`
Buat track baru.

```sql
INSERT INTO exam_tracks (name, slug, icon, sort_order, status)
VALUES ('PPPK', 'pppk', 'icon-pppk', 3, 'active');

SELECT id, slug, name, icon, status, sort_order, 0 AS question_count
FROM exam_tracks WHERE slug = 'pppk';
```

---

### `PUT /admin/hierarchy/tracks/{id}`
Update track.

```sql
UPDATE exam_tracks
SET name = 'CPNS 2025', slug = 'cpns-2025', icon = 'icon-cpns', sort_order = 1, status = 'active'
WHERE id = 1;
```

---

### `DELETE /admin/hierarchy/tracks/{id}`
Hapus track beserta semua hierarchy di bawahnya.

```sql
-- Validasi tidak ada children (jika strict mode)
SELECT COUNT(*) FROM categories WHERE track_id = 1;

-- Cascade delete
DELETE tp FROM topics tp
JOIN subcategories sc ON sc.id = tp.subcategory_id
JOIN categories c ON c.id = sc.category_id
WHERE c.track_id = 1;

DELETE sc FROM subcategories sc
JOIN categories c ON c.id = sc.category_id
WHERE c.track_id = 1;

DELETE FROM categories WHERE track_id = 1;
DELETE FROM exam_tracks WHERE id = 1;
```

---

### `POST /admin/hierarchy/tracks/reorder`
Reorder urutan tracks.

```sql
UPDATE exam_tracks SET sort_order = 1 WHERE id = 3;
UPDATE exam_tracks SET sort_order = 2 WHERE id = 1;
UPDATE exam_tracks SET sort_order = 3 WHERE id = 2;
```

---

### `POST /admin/hierarchy/categories`
Buat category baru.

```sql
INSERT INTO categories (track_id, name, slug, sort_order)
VALUES (1, 'TWK', 'twk', 1);

SELECT id, track_id, slug, name, sort_order, 0 AS question_count
FROM categories WHERE track_id = 1 AND slug = 'twk';
```

---

### `POST /admin/hierarchy/subcategories`
Buat subcategory baru.

```sql
INSERT INTO subcategories (category_id, name, slug, sort_order)
VALUES (3, 'Pancasila', 'pancasila', 1);
```

---

### `POST /admin/hierarchy/topics`
Buat topic baru.

```sql
INSERT INTO topics (subcategory_id, name, slug, sort_order)
VALUES (5, 'Sila ke-1', 'sila-ke-1', 1);
```

---

### `POST /admin/hierarchy/tags`
Buat tag baru.

```sql
INSERT INTO tags (label, slug) VALUES ('Ketuhanan', 'ketuhanan');

SELECT id, slug, label FROM tags WHERE slug = 'ketuhanan';
```

---

### `POST /admin/hierarchy/tags/merge`
Merge tags — pindahkan semua relasi dari tag sumber ke tag target.

```sql
-- Pindahkan relasi question_tags (hindari duplikat)
UPDATE question_tags SET tag_id = 5   -- target_tag_id
WHERE tag_id = 12                     -- source_tag_id
  AND question_id NOT IN (
    SELECT question_id FROM (
        SELECT question_id FROM question_tags WHERE tag_id = 5
    ) AS existing
);

-- Hapus sisa relasi tag sumber
DELETE FROM question_tags WHERE tag_id = 12;

-- Hapus tag sumber
DELETE FROM tags WHERE id = 12;
```

---

### `GET /admin/stats/questions`
Statistik soal (total, per status, per difficulty, per track).

```sql
SELECT COUNT(*) AS total FROM soal;

SELECT status, COUNT(*) AS count FROM soal GROUP BY status;

SELECT difficulty_est, COUNT(*) AS count FROM soal GROUP BY difficulty_est;

SELECT t.name AS track, COUNT(s.id) AS count
FROM soal s
LEFT JOIN exam_tracks t ON t.id = s.track_id
GROUP BY t.name;

SELECT COUNT(*) AS untagged FROM soal WHERE track_id IS NULL;

-- 10 soal terbaru
SELECT id, SUBSTRING(soal, 1, 80) AS soal_snippet, status,
       COALESCE(track_id, '') AS track_id, created_at
FROM soal ORDER BY created_at DESC LIMIT 10;

-- Soal dengan difficulty mismatch
SELECT id, SUBSTRING(soal, 1, 80) AS soal_snippet, difficulty_est,
       COALESCE(difficulty_calc, '') AS difficulty_calc
FROM soal
WHERE difficulty_calc IS NOT NULL AND difficulty_calc != difficulty_est
LIMIT 20;

-- Coverage per topic
SELECT t.id AS topic_id, t.name AS topic_name,
    SUM(CASE WHEN s.difficulty_est = 'easy' THEN 1 ELSE 0 END) AS easy,
    SUM(CASE WHEN s.difficulty_est = 'medium' THEN 1 ELSE 0 END) AS medium,
    SUM(CASE WHEN s.difficulty_est = 'hard' THEN 1 ELSE 0 END) AS hard
FROM topics t
LEFT JOIN soal s ON s.topic_id = t.id
GROUP BY t.id, t.name
ORDER BY (easy + medium + hard) ASC
LIMIT 30;
```

---

## 21. Admin — Passages

### `GET /admin/passages`
List passages (paginated).

```sql
SELECT COUNT(*) FROM passages;

SELECT * FROM passages ORDER BY id DESC LIMIT 20 OFFSET 0;
```

---

### `POST /admin/passages`
Buat passage baru.

```sql
INSERT INTO passages (content, title, source, language)
VALUES ('Teks bacaan panjang...', 'Teks Pancasila', 'Buku PKN', 'id');
```

---

### `GET /admin/passages/{id}`
Detail passage + daftar soal yang memakai passage ini.

```sql
SELECT * FROM passages WHERE id = 3;

SELECT id FROM soal WHERE passage_id = 3 ORDER BY id;
```

---

### `PUT /admin/passages/{id}`
Update passage.

```sql
UPDATE passages
SET content = 'Teks baru...', title = 'Judul baru', source = 'Sumber baru',
    language = 'id', updated_at = NOW()
WHERE id = 3;
```

---

### `DELETE /admin/passages/{id}`
Hapus passage.

```sql
DELETE FROM passages WHERE id = 3;
```

---

## 22. Admin — Simulasi Ujian

### `GET /admin/simulasi-ujian`
List simulasi (admin, dengan statistik).

```sql
SELECT
    s.id, s.nama_simulasi, s.deskripsi, s.paket_soal_id,
    ps.nama_paket_soal AS paket_soal_nama,
    s.duration_minutes, s.total_questions, s.passing_score,
    s.is_premium, s.max_attempts, s.is_active,
    s.created_at, s.updated_at,
    COALESCE(stats.total_attempts, 0) AS total_attempts,
    stats.avg_score,
    stats.pass_rate
FROM exam_simulations s
LEFT JOIN paket_soal ps ON ps.id = s.paket_soal_id
LEFT JOIN (
    SELECT simulasi_id,
           COUNT(*) AS total_attempts,
           AVG(score) AS avg_score,
           AVG(CASE WHEN is_passed THEN 100.0 ELSE 0.0 END) AS pass_rate
    FROM simulasi_user_attempts
    WHERE completed_at IS NOT NULL
    GROUP BY simulasi_id
) stats ON stats.simulasi_id = s.id
WHERE s.is_active = TRUE
ORDER BY s.created_at DESC;
```

---

### `POST /admin/simulasi-ujian`
Buat simulasi ujian baru.

```sql
INSERT INTO exam_simulations (
    nama_simulasi, deskripsi, paket_soal_id, generation_mode, generation_config,
    duration_minutes, total_questions, passing_score, is_premium, max_attempts, is_active
)
VALUES (
    'Try Out CPNS 2025 - Sesi 1', 'Simulasi resmi 100 soal CPNS',
    5, 'from_paket', NULL,
    90, 100, 70, FALSE, 3, TRUE
);
```

---

### `GET /admin/simulasi-ujian/stats`
Statistik simulasi secara keseluruhan.

```sql
SELECT
    (SELECT COUNT(*) FROM exam_simulations WHERE is_active = TRUE) AS total_active,
    (SELECT COUNT(*) FROM simulasi_user_attempts
        WHERE completed_at IS NOT NULL AND completed_at >= CURRENT_DATE) AS attempts_today,
    (SELECT COUNT(*) FROM simulasi_user_attempts
        WHERE completed_at IS NOT NULL
          AND completed_at >= DATE_SUB(CURRENT_DATE, INTERVAL 7 DAY)) AS attempts_week,
    CAST(COALESCE(
        (SELECT AVG(CASE WHEN is_passed THEN 100 ELSE 0 END)
         FROM simulasi_user_attempts WHERE completed_at IS NOT NULL),
        0
    ) AS SIGNED) AS avg_pass_rate;
```

---

### `PUT /admin/simulasi-ujian/{id}`
Update simulasi.

```sql
UPDATE exam_simulations
SET nama_simulasi = 'Try Out CPNS 2025 - Revisi',
    duration_minutes = 120,
    passing_score = 75,
    is_active = TRUE
WHERE id = 1;
```

---

### `DELETE /admin/simulasi-ujian/{id}`
Soft delete simulasi (set is_active = FALSE).

```sql
UPDATE exam_simulations SET is_active = FALSE WHERE id = 1;
```

---

### `GET /admin/simulasi-ujian/{id}/attempts`
Semua attempt untuk simulasi ini (admin).

```sql
SELECT a.id, a.simulasi_id, a.user_id, a.quiz_session_id, a.attempt_number,
       a.score, a.is_passed, a.completed_at, a.created_at,
       u.email AS user_email, u.display_name AS user_display_name
FROM simulasi_user_attempts a
LEFT JOIN users u ON u.id = a.user_id
WHERE a.simulasi_id = 1
ORDER BY a.created_at DESC;
```

---

## 23. Admin — Analytics

### `GET /admin/analytics/dashboard`
Statistik dashboard utama.

```sql
-- Total users
SELECT COUNT(*) FROM users WHERE deleted_at IS NULL;

-- New users hari ini
SELECT COUNT(*) FROM users WHERE DATE(created_at) = CURDATE() AND deleted_at IS NULL;

-- Quiz sessions hari ini
SELECT COUNT(*) FROM quiz_sessions WHERE DATE(created_at) = CURDATE() AND is_completed = TRUE;

-- Revenue bulan ini
SELECT COALESCE(SUM(amount), 0) FROM payment_transactions
WHERE status = 'paid' AND DATE_FORMAT(created_at, '%Y-%m') = DATE_FORMAT(NOW(), '%Y-%m');

-- Premium users aktif
SELECT COUNT(DISTINCT user_id) FROM user_subscriptions
WHERE status = 'active' AND (end_date IS NULL OR end_date > NOW());
```

---

### `GET /admin/analytics/users`
Analitik user (registrasi per hari/minggu/bulan).

```sql
-- Registrasi per hari (30 hari terakhir)
SELECT DATE(created_at) AS tanggal, COUNT(*) AS total
FROM users
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 30 DAY) AND deleted_at IS NULL
GROUP BY DATE(created_at)
ORDER BY tanggal;

-- User aktif (punya quiz session dalam 7 hari terakhir)
SELECT COUNT(DISTINCT user_id) FROM quiz_sessions
WHERE updated_at >= DATE_SUB(NOW(), INTERVAL 7 DAY) AND is_completed = TRUE;
```

---

### `GET /admin/analytics/quiz-sessions`
Analitik quiz sessions.

```sql
-- Quiz per kategori
SELECT kategori_soal, COUNT(*) AS total, AVG(score) AS avg_score
FROM quiz_sessions WHERE is_completed = TRUE
GROUP BY kategori_soal ORDER BY total DESC;

-- Quiz per hari (30 hari)
SELECT DATE(updated_at) AS tanggal, COUNT(*) AS total
FROM quiz_sessions
WHERE is_completed = TRUE AND updated_at >= DATE_SUB(NOW(), INTERVAL 30 DAY)
GROUP BY DATE(updated_at)
ORDER BY tanggal;
```

---

### `GET /admin/analytics/revenue`
Analitik revenue.

```sql
-- Revenue per bulan (12 bulan terakhir)
SELECT DATE_FORMAT(created_at, '%Y-%m') AS bulan,
       COUNT(*) AS total_transaksi,
       SUM(amount) AS total_revenue
FROM payment_transactions
WHERE status = 'paid'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 12 MONTH)
GROUP BY DATE_FORMAT(created_at, '%Y-%m')
ORDER BY bulan;

-- Revenue per plan
SELECT pp.name AS plan_name, COUNT(*) AS total, SUM(pt.amount) AS revenue
FROM payment_transactions pt
JOIN premium_plans pp ON pp.id = pt.plan_id
WHERE pt.status = 'paid'
GROUP BY pp.name ORDER BY revenue DESC;
```

---

## 24. Admin — Alerts

### `GET /admin/alerts`
List alert dengan filter + pagination.

```sql
SELECT * FROM admin_alerts
WHERE is_read = FALSE
ORDER BY created_at DESC
LIMIT 20 OFFSET 0;
```

---

### `PUT /admin/alerts/{id}/read`
Tandai alert sudah dibaca.

```sql
UPDATE admin_alerts SET is_read = TRUE, read_at = NOW() WHERE id = 10;
```

---

### `PUT /admin/alerts/read-all`
Tandai semua alert sudah dibaca.

```sql
UPDATE admin_alerts SET is_read = TRUE, read_at = NOW() WHERE is_read = FALSE;
```

---

## 25. Internal

### `POST /internal/soal/bulk-import`
Import soal massal via API internal (IP + API Key terbatas).

```sql
INSERT INTO dbquizapp.soal (
    soal, question_type, opt1, opt2, opt3, opt4, correct_answer, solution,
    modul, pelajaran, track_id, category_id, subcategory_id, topic_id,
    difficulty_est, status, source
)
VALUES (...); -- Looping per soal dari payload

-- Set tags per soal
DELETE FROM question_tags WHERE question_id = ?;
INSERT IGNORE INTO question_tags (question_id, tag_id) VALUES (?, ?);
```

---

### `GET /internal/soal/seo`
Ambil soal untuk keperluan SEO/sitemap.

```sql
SELECT
    s.id, s.soal, s.opt1, s.opt2, s.opt3, s.opt4,
    s.correct_answer, s.solution,
    COALESCE(t.slug,  'lainnya') AS track_slug,
    COALESCE(t.name,  'Lainnya') AS track_name,
    COALESCE(c.slug,  'umum')    AS category_slug,
    COALESCE(c.name,  'Umum')    AS category_name,
    COALESCE(tp.name, '')        AS topic_name
FROM soal s
LEFT JOIN exam_tracks  t  ON t.id  = s.track_id
LEFT JOIN categories   c  ON c.id  = s.category_id
LEFT JOIN topics       tp ON tp.id = s.topic_id
WHERE s.status = 'active'
  AND s.solution IS NOT NULL
  AND s.solution <> ''
ORDER BY s.id;
```

---

## Query Utilitas (Berguna untuk Analisis Data)

```sql
-- Cek expired subscriptions
SELECT COUNT(*) FROM user_subscriptions
WHERE status = 'active' AND end_date IS NOT NULL AND end_date < NOW();

-- Update expired subscriptions (job periodik)
UPDATE dbquizapp.user_subscriptions
SET status = 'expired'
WHERE status = 'active' AND end_date IS NOT NULL AND end_date < NOW();

-- Cleanup quiz sessions yang tidak aktif > 48 jam
DELETE FROM quiz_sessions
WHERE is_completed = FALSE AND updated_at < DATE_SUB(NOW(), INTERVAL 48 HOUR);

-- Hapus expired sessions
DELETE FROM sessions WHERE expires_at <= NOW();

-- Cek difficulty mismatch (soal yang perlu di-review)
SELECT id, SUBSTRING(soal, 1, 80) AS soal_snippet, difficulty_est, difficulty_calc,
       attempt_count, p_value
FROM soal
WHERE difficulty_calc IS NOT NULL AND difficulty_calc != difficulty_est
ORDER BY attempt_count DESC
LIMIT 50;

-- Top soal yang paling sering dilaporkan
SELECT qr.question_id,
        LEFT(s.soal, 100) AS question_text,
        SUM(CASE WHEN qr.status = 'pending' THEN 1 ELSE 0 END) AS pending_count,
        COUNT(*) AS total_count
FROM question_reports qr
JOIN soal s ON s.id = qr.question_id
GROUP BY qr.question_id, s.soal
ORDER BY pending_count DESC
LIMIT 20;

-- Distribusi skor quiz (histogram)
SELECT
    CASE
        WHEN score < 40 THEN '0-39'
        WHEN score < 60 THEN '40-59'
        WHEN score < 80 THEN '60-79'
        ELSE '80-100'
    END AS range_skor,
    COUNT(*) AS total
FROM quiz_sessions
WHERE is_completed = TRUE
GROUP BY range_skor
ORDER BY range_skor;

-- User yang paling aktif (top 10 quiz terbanyak)
SELECT u.display_name, u.email, COUNT(qs.id) AS total_quiz, AVG(qs.score) AS avg_score
FROM quiz_sessions qs
JOIN users u ON u.id = qs.user_id
WHERE qs.is_completed = TRUE
GROUP BY u.id ORDER BY total_quiz DESC LIMIT 10;

-- Revenue per hari (7 hari terakhir)
SELECT DATE(created_at) AS tanggal, COUNT(*) AS transaksi, SUM(amount) AS revenue
FROM payment_transactions
WHERE status = 'paid' AND created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
GROUP BY DATE(created_at) ORDER BY tanggal;
```

---

*Dokumen ini di-generate dari source code quiz-backend pada 2026-05-30.
Untuk update, re-generate dari controller/*.rs dan dao/*.rs.*
