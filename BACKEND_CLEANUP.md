# Backend Cleanup — Dokumentasi Perubahan
> Tanggal: 2026-03-04
> Tujuan: Membersihkan endpoint yang tidak dipakai, menghapus duplikat, dan mengaktifkan maintenance task

---

## Ringkasan Perubahan

| File | Perubahan | Alasan |
|------|-----------|--------|
| `src/controller/auth_controller.rs` | Hapus `POST /signup` dan `POST /auth/v1/token` | Tidak dipakai — hanya Google OAuth yang aktif |
| `src/controller/soal_controller.rs` | Hapus `GET /kumpulan-soal`, `POST /soal`, `PUT /set-quiz-premium/{id}` | Fungsi admin, sudah ada di `/admin/soal` dengan middleware admin |
| `src/controller/license_controller.rs` | Hapus `POST /license-public/verify-mock` | Endpoint test/mock, berbahaya di production |
| `src/controller/premium_controller.rs` | Hapus `GET /premium/quiz-access/check/{id}` dan `GET /premium/check-quiz-access/{id}` | Duplikat dari `GET /check-quiz-access/{id}` di soal_controller |
| `src/main.rs` | Aktifkan session cleanup background task | Task ini sudah ada tapi di-comment |
| `src/docs.rs` | Update OpenAPI docs — hapus referensi ke endpoint yang dihapus | Mengikuti perubahan controller |

---

## Detail Perubahan

### 1. `auth_controller.rs` — Hapus Email Auth

**Yang dihapus:**
- `POST /signup` — registrasi email+password via Supabase
- `POST /auth/v1/token` — login email+password via Supabase

**Alasan:**
Aplikasi menggunakan Google OAuth sebagai satu-satunya metode autentikasi pengguna. Kedua endpoint ini tidak pernah dipanggil oleh frontend web maupun mobile. Membiarkannya aktif menciptakan attack surface yang tidak perlu.

**Yang tetap ada:**
- `POST /auth/login` — admin login (bypass Supabase, cek role di database)
- `POST /auth/google/callback` — Google OAuth utama
- `POST /auth/logout` — invalidate session
- `GET /auth/sessions/{user_id}` — lihat sesi aktif user
- `POST /auth/validate-session` — validasi token

**Import yang dibersihkan:**
```rust
// SEBELUM
use crate::model::{SignUpRequest, LoginRequest, AuthResponse, SupabaseUser};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm, decode, DecodingKey, Validation};
use actix_web::{post, web, HttpResponse, Responder, HttpRequest, get, delete};

// SESUDAH
use crate::model::{LoginRequest, AuthResponse, SupabaseUser};
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use actix_web::{post, web, HttpResponse, Responder, HttpRequest, get};
```

---

### 2. `soal_controller.rs` — Pindahkan Admin Endpoints

**Yang dihapus:**
- `GET /kumpulan-soal` — ambil semua soal (tanpa filter, tanpa paginasi)
- `POST /soal` — buat soal baru
- `PUT /set-quiz-premium/{paket_soal_id}` — ubah status premium paket soal

**Alasan:**
Ketiga endpoint ini adalah operasi admin yang **sudah tersedia** di `/admin/soal` dengan perlindungan `AdminMiddleware`. Versi di soal_controller tidak memiliki validasi role admin yang proper (`// TODO: Check if user has admin privileges`).

**Endpoint pengganti (gunakan ini):**
| Dihapus | Gantinya |
|---------|----------|
| `GET /kumpulan-soal` | `GET /admin/soal` (dengan pagination + filter) |
| `POST /soal` | `POST /admin/soal` |
| `PUT /set-quiz-premium/{id}` | `PUT /admin/packages/{id}` (field `is_premium`) |

**Import yang dibersihkan:**
```rust
// SEBELUM
use actix_web::{get, post, put, web, HttpResponse, Responder, HttpRequest};
use crate::model::CreateSoalRequest;
use serde::{Deserialize, Serialize};

// SESUDAH
use actix_web::{get, web, HttpResponse, Responder, HttpRequest};
// (tidak ada import model yang dihapus dari fungsi yang tersisa)
```

---

### 3. `license_controller.rs` — Hapus Mock Endpoint

**Yang dihapus:**
- `POST /license-public/verify-mock` — verifikasi lisensi palsu untuk testing

**Alasan:**
Endpoint mock ini bisa dieksploitasi di production untuk mendapatkan subscription gratis tanpa benar-benar membayar. Untuk testing, gunakan environment development dengan Mayar sandbox.

**Yang tetap ada:**
- `GET /license/payment-link/{plan_id}` — generate link pembayaran (butuh auth)
- `GET /license/user-licenses` — lihat lisensi milik user (butuh auth)
- `POST /license-public/verify` — verifikasi dan aktivasi lisensi via Mayar (tanpa auth, untuk activation page)

---

### 4. `premium_controller.rs` — Hapus Duplikat Check Access

**Yang dihapus:**
- `GET /premium/quiz-access/check/{paket_soal_id}` — fungsi `check_quiz_access`
- `GET /premium/check-quiz-access/{paket_soal_id}` — fungsi `check_premium_quiz_access`

**Alasan:**
Kedua endpoint ini adalah duplikat dari `GET /check-quiz-access/{paket_soal_id}` yang ada di `soal_controller.rs`. Memiliki 3 endpoint dengan fungsi yang sama membingungkan dan rawan inkonsistensi.

**Endpoint yang dipertahankan (canonical):**
```
GET /check-quiz-access/{paket_soal_id}
```
Response format:
```json
{
  "success": true,
  "can_access": true | false,
  "is_premium": true | false,
  "message": "...",
  "available_plans": [...]  // hanya jika can_access = false
}
```

---

### 5. `main.rs` — Aktifkan Session Cleanup

**Perubahan:**
```rust
// SEBELUM (dikomentari)
// let app_state_clone = app_state.clone();
// tokio::spawn(async move {
//     let mut interval = time::interval(Duration::from_secs(3600));
//     ...
// });

// SESUDAH (aktif)
let app_state_clone = app_state.clone();
tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(3600)); // setiap 1 jam
    loop {
        interval.tick().await;
        if let Err(e) = app_state_clone.context.sessions.delete_expired_sessions().await {
            eprintln!("Failed to clean up expired sessions: {:?}", e);
        }
    }
});
```

**Alasan:**
Session yang expired harus dibersihkan secara berkala agar tabel `sessions` tidak terus membesar. Task ini sudah diimplementasikan tapi lupa di-uncomment.

---

## API Endpoint — Sebelum vs Sesudah

### Endpoint yang DIHAPUS (jangan pakai lagi)

| Method | Path | Catatan |
|--------|------|---------|
| POST | `/signup` | Gunakan Google OAuth |
| POST | `/auth/v1/token` | Gunakan Google OAuth |
| GET | `/kumpulan-soal` | Gunakan `GET /admin/soal` |
| POST | `/soal` | Gunakan `POST /admin/soal` |
| PUT | `/set-quiz-premium/{id}` | Gunakan `PUT /admin/packages/{id}` |
| POST | `/license-public/verify-mock` | Endpoint mock, dihapus permanen |
| GET | `/premium/quiz-access/check/{id}` | Gunakan `GET /check-quiz-access/{id}` |
| GET | `/premium/check-quiz-access/{id}` | Gunakan `GET /check-quiz-access/{id}` |

### Endpoint AKTIF setelah cleanup

#### Auth
```
POST /auth/login                     — Admin login
POST /auth/google/callback           — Google OAuth callback
POST /auth/logout                    — Logout
GET  /auth/sessions/{user_id}        — Lihat sesi aktif
POST /auth/validate-session          — Validasi token
```

#### Soal & Paket Soal
```
GET  /soal/{id}                                          — Ambil soal by ID
GET  /paket-soal-response/{kategori}/{paket}             — Soal dalam paket (dengan akses check)
GET  /paket-soal-response/{kategori}                     — List paket dalam kategori
GET  /listpaketsoal                                      — Semua paket soal (list ringkas)
GET  /listpaketsoallengkap                               — Semua paket soal (dengan info harga)
GET  /check-quiz-access/{paket_soal_id}                  — Cek akses user ke paket soal
```

#### Kategori
```
GET  /kategori                       — List semua kategori
GET  /kategori/{id}                  — Detail kategori
```

#### User
```
POST /user/check-phone               — Cek apakah user punya nomor HP
POST /user/update-phone              — Update nomor HP user
```

#### Premium & Subscription
```
GET  /premium/plans                  — List paket berlangganan
GET  /premium/plans/{id}             — Detail paket berlangganan
GET  /premium/subscriptions          — Subscription milik user
GET  /premium/subscriptions/active   — Subscription aktif user
GET  /premium/subscriptions/{id}     — Detail subscription
POST /premium/subscriptions/{id}/cancel — Batalkan subscription
GET  /premium/check-status           — Status premium user + detail subscription
```

#### Payment
```
POST /payment/create                 — Buat transaksi baru
GET  /payment/status/{id}            — Status pembayaran
POST /payment/webhook/mayar          — Webhook dari Mayar
```

#### License
```
GET  /license/payment-link/{plan_id} — Generate link pembayaran Mayar
GET  /license/user-licenses          — Lihat semua lisensi user
POST /license-public/verify          — Aktivasi lisensi dari Mayar
```

#### Quiz Session
```
POST /quiz-session                   — Mulai sesi quiz
GET  /quiz-session/{id}              — Ambil data sesi
PUT  /quiz-session/{id}/progress     — Simpan progress jawaban
POST /quiz-session/{id}/complete     — Selesaikan quiz
GET  /quiz-session/user/history      — Riwayat quiz user
```

#### Admin (butuh header admin JWT)
```
GET/POST/PUT/DELETE /admin/users/...
GET/POST/PUT/DELETE /admin/kategori/...
GET/POST/PUT/DELETE /admin/soal/...           (termasuk CSV import)
GET/POST/PUT/DELETE /admin/packages/...
GET/POST/PUT/DELETE /admin/paket-soal-items/...
GET                 /admin/analytics/...
```

---

## Migration Guide untuk Client (Web & Mobile)

Jika ada kode di frontend/mobile yang memanggil endpoint yang dihapus:

### Auth
```typescript
// SEBELUM (jangan pakai)
await http.post('/signup', { email, password, display_name })
await http.post('/auth/v1/token', { email, password })

// SESUDAH
// Hanya gunakan Google OAuth flow
window.location.href = getGoogleOAuthUrl()
```

### Soal admin
```typescript
// SEBELUM (jangan pakai dari client biasa)
await http.get('/kumpulan-soal')
await http.post('/soal', newSoal)
await http.put(`/set-quiz-premium/${id}`, { is_premium: true })

// SESUDAH (khusus admin panel)
await http.get('/admin/soal?page=1&limit=20')
await http.post('/admin/soal', newSoal)
await http.put(`/admin/packages/${id}`, { is_premium: true })
```

### Check quiz access
```typescript
// SEBELUM (semua ini tidak lagi tersedia)
await http.get(`/premium/quiz-access/check/${paketId}`)
await http.get(`/premium/check-quiz-access/${paketId}`)

// SESUDAH (satu endpoint canonical)
await http.get(`/check-quiz-access/${paketId}`)
```

---

## Status Kompilasi

Setelah semua perubahan:
```
$ cargo check
warning: ... (96 warnings, mostly unused imports dari kode lama)
Finished — 0 errors
```

> Warnings yang ada adalah dari kode yang tidak diubah (redis_service.rs, admin_middleware.rs).
> Bisa dibersihkan di sprint berikutnya, tidak mempengaruhi fungsionalitas.

---

*Dibuat: 2026-03-04 | Backend: Rust Actix-Web*
