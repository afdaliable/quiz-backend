/**
 * k6 Load Test — Quiz Backend
 *
 * Skenario: user login → ambil paket soal → mulai sesi → simpan progress → selesaikan sesi
 *
 * Jalankan:
 *   k6 run quiz-loadtest.js
 *   k6 run --vus 200 --duration 5m quiz-loadtest.js
 *   k6 run --out json=hasil.json quiz-loadtest.js
 *   k6 run --out influxdb=http://localhost:8086/k6 quiz-loadtest.js  (untuk Grafana)
 *
 * Env vars:
 *   BASE_URL   — default: http://localhost:8080
 *   SCENARIO   — 'smoke' | 'load' | 'stress' | 'spike' | 'soak'  (default: load)
 */

import http from 'k6/http'
import { check, sleep, group } from 'k6'
import { SharedArray } from 'k6/data'
import { Trend, Rate, Counter } from 'k6/metrics'

// ── Config ────────────────────────────────────────────────────────────────────

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080'
const SCENARIO  = __ENV.SCENARIO  || 'load'

// ── Custom Metrics ────────────────────────────────────────────────────────────

const loginDuration    = new Trend('quiz_login_duration',    true)
const sessionDuration  = new Trend('quiz_session_duration',  true)
const completeDuration = new Trend('quiz_complete_duration', true)
const errorRate        = new Rate('quiz_error_rate')
const sessionCounter   = new Counter('quiz_sessions_completed')

// ── Test Scenarios ────────────────────────────────────────────────────────────

const scenarios = {
  // Smoke: 1 user, 1 menit — pastikan script jalan tanpa error
  smoke: {
    stages: [
      { duration: '1m', target: 1 },
    ],
    thresholds: {
      'quiz_error_rate':          ['rate<0.01'],
      'http_req_duration':        ['p(95)<1000'],
    },
  },

  // Load: ramp ke 100 user, tahan 5 menit — simulasi normal load
  load: {
    stages: [
      { duration: '2m', target: 50  },  // ramp up
      { duration: '5m', target: 100 },  // peak load
      { duration: '1m', target: 0   },  // ramp down
    ],
    thresholds: {
      'quiz_error_rate':          ['rate<0.01'],
      'http_req_duration':        ['p(95)<500'],
      'quiz_login_duration':      ['p(95)<300'],
      'quiz_complete_duration':   ['p(95)<800'],
    },
  },

  // Stress: cari breaking point — naikkan sampai error muncul
  stress: {
    stages: [
      { duration: '2m',  target: 100  },
      { duration: '3m',  target: 300  },
      { duration: '3m',  target: 600  },
      { duration: '3m',  target: 1000 },
      { duration: '2m',  target: 0    },
    ],
    thresholds: {
      'quiz_error_rate': ['rate<0.05'],  // toleransi lebih longgar untuk stress
    },
  },

  // Spike: simulasi tryout massal — lonjakan tiba-tiba
  spike: {
    stages: [
      { duration: '30s', target: 10   },  // normal
      { duration: '1m',  target: 1000 },  // spike!
      { duration: '3m',  target: 1000 },  // tahan
      { duration: '1m',  target: 10   },  // turun
      { duration: '2m',  target: 0    },
    ],
    thresholds: {
      'quiz_error_rate': ['rate<0.05'],
    },
  },

  // Soak: cari memory leak — tahan 1 jam dengan load sedang
  soak: {
    stages: [
      { duration: '5m',  target: 100 },
      { duration: '50m', target: 100 },
      { duration: '5m',  target: 0   },
    ],
    thresholds: {
      'quiz_error_rate':    ['rate<0.01'],
      'http_req_duration':  ['p(95)<500'],
    },
  },
}

export const options = {
  stages:     scenarios[SCENARIO].stages,
  thresholds: scenarios[SCENARIO].thresholds,
}

// ── Test Users ────────────────────────────────────────────────────────────────
//
// Buat dulu di DB sebelum test:
//   INSERT INTO users (id, email, display_name, password_hash, role, created_at)
//   SELECT
//     UUID(),
//     CONCAT('loadtest', seq, '@test.com'),
//     CONCAT('Load Test User ', seq),
//     '$2b$10$Kz8Xc1N5vQ9mR3pL2hJ7uOa0YwE4bGdFiTs6nVcWx1yZ3jMqPo8Ie',  -- bcrypt hash of 'Loadtest123!'
//     'user',
//     NOW()
//   FROM (
//     SELECT a.N + b.N * 10 + 1 AS seq
//     FROM (SELECT 0 AS N UNION SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4
//           UNION SELECT 5 UNION SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9) a,
//          (SELECT 0 AS N UNION SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4
//           UNION SELECT 5 UNION SELECT 6 UNION SELECT 7 UNION SELECT 8 UNION SELECT 9) b
//     ORDER BY seq
//   ) seq_table;
//
// Password plain-text: Loadtest123!

// Password boleh isi apa saja — backend tidak mengecek password untuk role admin
const TEST_USERS = new SharedArray('users', function () {
  return Array.from({ length: 100 }, (_, i) => ({
    email:    `loadtest${i + 1}@test.com`,
    password: 'test',
  }))
})

// ── Helpers ───────────────────────────────────────────────────────────────────

const JSON_HEADERS = { 'Content-Type': 'application/json' }

function authHeaders(token) {
  return { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }
}

function randomInt(min, max) {
  return Math.floor(Math.random() * (max - min + 1)) + min
}

// Simulasi jawaban acak: null = belum dijawab, 0-4 = opt1-opt5
function randomAnswers(count) {
  return Array.from({ length: count }, () =>
    Math.random() < 0.8 ? randomInt(0, 4) : null
  )
}

// ── Token cache per VU (login sekali, reuse untuk semua iterasi) ──────────────
let cachedToken = null

// ── Main Flow ─────────────────────────────────────────────────────────────────

export default function () {
  const user = TEST_USERS[__VU % TEST_USERS.length]
  let sessionId, jumlahSoal

  // ── Step 1: Login (hanya sekali per VU) ────────────────────────────────────
  if (!cachedToken) {
    group('1_login', () => {
      const t0  = Date.now()
      const res = http.post(
        `${BASE_URL}/auth/login`,
        JSON.stringify({ email: user.email, password: user.password }),
        { headers: JSON_HEADERS },
      )
      loginDuration.add(Date.now() - t0)

      const ok = check(res, {
        'login: status 200': r => r.status === 200,
        'login: ada token':  r => !!r.json('token'),
      })
      if (!ok) { errorRate.add(1); return }

      cachedToken = res.json('token')
      errorRate.add(0)
    })
  }

  const token = cachedToken
  if (!token) return
  sleep(randomInt(1, 2))

  // ── Step 2: Ambil daftar paket soal ────────────────────────────────────────
  let paketSoalId, kategoriSoal, namaPaketSoal
  group('2_list_paket', () => {
    const res = http.get(`${BASE_URL}/listpaketsoal`, { headers: authHeaders(token) })

    const ok = check(res, {
      'listpaketsoal: status 200': r => r.status === 200,
      'listpaketsoal: ada data':   r => Array.isArray(r.json()) && r.json().length > 0,
    })
    if (!ok) { errorRate.add(1); return }

    const list = res.json()
    // Pilih paket yang tidak premium secara acak
    const freePackages = list.filter(p => !p.is_premium)
    const pkg = freePackages.length > 0
      ? freePackages[randomInt(0, freePackages.length - 1)]
      : list[randomInt(0, list.length - 1)]

    paketSoalId   = pkg.id_nama_paket_soal
    kategoriSoal  = pkg.kategori_soal
    namaPaketSoal = pkg.nama_paket_soal
    jumlahSoal    = pkg.jumlah_soal || 10
    errorRate.add(0)
  })

  if (!paketSoalId) return
  sleep(randomInt(1, 3))

  // ── Step 3: Mulai sesi quiz ────────────────────────────────────────────────
  group('3_start_session', () => {
    const t0  = Date.now()
    const res = http.post(
      `${BASE_URL}/quiz-session/start`,
      JSON.stringify({
        paket_soal_id:    paketSoalId,
        kategori_soal:    kategoriSoal,
        nama_paket_soal:  namaPaketSoal,
        total_time:       5400,  // 90 menit
      }),
      { headers: authHeaders(token) },
    )
    sessionDuration.add(Date.now() - t0)

    const ok = check(res, {
      'start_session: status 200': r => r.status === 200,
      'start_session: ada id':     r => !!r.json('id'),
    })
    if (!ok) { errorRate.add(1); return }

    sessionId = res.json('id')
    errorRate.add(0)
  })

  if (!sessionId) return
  sleep(randomInt(2, 5))

  // ── Step 4: Simpan progress (simulasi menjawab sebagian soal) ──────────────
  group('4_save_progress', () => {
    const answered = Math.min(Math.floor(jumlahSoal * 0.4), 20)  // jawab ~40%
    const res = http.put(
      `${BASE_URL}/quiz-session/${sessionId}/save`,
      JSON.stringify({
        current_question: answered,
        answers:          randomAnswers(jumlahSoal),
        marked_questions: Array.from({ length: jumlahSoal }, () => Math.random() < 0.1),
        time_remaining:   4800,
      }),
      { headers: authHeaders(token) },
    )

    check(res, { 'save_progress: status 200': r => r.status === 200 })
    if (res.status !== 200) errorRate.add(1)
    else errorRate.add(0)
  })

  sleep(randomInt(3, 8))

  // ── Step 5: Selesaikan sesi ───────────────────────────────────────────────
  group('5_complete_session', () => {
    const t0  = Date.now()
    const res = http.put(
      `${BASE_URL}/quiz-session/${sessionId}/complete`,
      JSON.stringify({
        answers:        randomAnswers(jumlahSoal),
        time_remaining: randomInt(0, 4000),
      }),
      { headers: authHeaders(token) },
    )
    completeDuration.add(Date.now() - t0)

    const ok = check(res, {
      'complete_session: status 200': r => r.status === 200,
      'complete_session: ada score':  r => r.json('score') !== undefined,
    })
    if (ok) {
      sessionCounter.add(1)
      errorRate.add(0)
    } else {
      errorRate.add(1)
    }
  })

  sleep(randomInt(1, 3))
}

// ── Teardown: ringkasan ───────────────────────────────────────────────────────

export function handleSummary(data) {
  const metrics = data.metrics
  const p95     = v => v ? v.values['p(95)'].toFixed(0) + 'ms' : 'N/A'
  const rate    = v => v ? (v.values.rate * 100).toFixed(2) + '%'   : 'N/A'

  console.log('\n══════════════════════════════════════')
  console.log('  QUIZ LOAD TEST — RINGKASAN')
  console.log('══════════════════════════════════════')
  console.log(`  Skenario       : ${SCENARIO}`)
  console.log(`  Base URL       : ${BASE_URL}`)
  console.log(`  Sesi selesai   : ${metrics.quiz_sessions_completed?.values.count ?? 0}`)
  console.log(`  Error rate     : ${rate(metrics.quiz_error_rate)}`)
  console.log(`  Login p95      : ${p95(metrics.quiz_login_duration)}`)
  console.log(`  Start sesi p95 : ${p95(metrics.quiz_session_duration)}`)
  console.log(`  Complete p95   : ${p95(metrics.quiz_complete_duration)}`)
  console.log(`  Semua req p95  : ${p95(metrics.http_req_duration)}`)
  console.log('══════════════════════════════════════\n')

  return {
    stdout: '',  // sudah print manual di atas
    'hasil-loadtest.json': JSON.stringify(data, null, 2),
  }
}
