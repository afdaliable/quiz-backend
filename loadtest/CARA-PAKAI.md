# Cara Menjalankan Load Test

## Langkah 1 — Buat test users di DB (sekali saja)

Dari PC kantor, jalankan di terminal:

```bash
# Test ke DEV database
mysql -h 100.124.237.85 -u afdaliable -p'love4JJI!' dbquizapp < setup-test-users.sql
```

Kalau mau test ke PROD:
```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp < setup-test-users.sql
```

---

## Langkah 2 — Install k6

```bash
sudo apt install k6
```

---

## Langkah 3A — Jalankan (output di terminal saja)

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend/loadtest

# Smoke test — 1 user, 1 menit (mulai dari sini)
BASE_URL=http://100.124.237.85:8080 SCENARIO=smoke k6 run quiz-loadtest.js

# Load test — 100 user concurrent
BASE_URL=http://100.124.237.85:8080 SCENARIO=load k6 run quiz-loadtest.js

# Stress test — cari breaking point
BASE_URL=http://100.124.237.85:8080 SCENARIO=stress k6 run quiz-loadtest.js
```

Hasil otomatis tersimpan ke `hasil-loadtest.json`.

---

## Langkah 3B — Jalankan dengan UI web (k6 Cloud, gratis)

1. Daftar di https://grafana.com/products/cloud/k6/
2. Login & ambil API token di dashboard
3. Dari terminal:

```bash
k6 login cloud --token <API_TOKEN_KAMU>

BASE_URL=http://100.124.237.85:8080 SCENARIO=load k6 cloud quiz-loadtest.js
```

4. Buka URL yang muncul di terminal → lihat grafik real-time di browser

---

## URL target

| Target | URL | Keterangan |
|---|---|---|
| DEV backend (via Tailscale) | `http://100.124.237.85:8080` | Test dulu ke sini |
| PROD backend (via Tailscale) | `http://100.87.162.99:PORT` | Setelah DEV OK |
| PROD via Cloudflare | `https://domain-kamu.com` | Full stack test |

---

## Setelah selesai testing — hapus test users

```bash
mysql -h 100.124.237.85 -u afdaliable -p'love4JJI!' dbquizapp \
  -e "DELETE FROM users WHERE email LIKE 'loadtest%@test.com';"
```
