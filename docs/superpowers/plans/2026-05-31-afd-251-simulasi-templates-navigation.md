# AFD-251: Simulasi Templates Navigation Mode & Komposisi Soal

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Koreksi komposisi soal SKD/RBB/STAN, tambah template PPPK & LPDP, dan implementasi field `navigation_mode` + `section_duration_minutes` di seluruh stack (DB → BE Rust → Admin FE).

**Architecture:** ALTER TABLE dulu (kolom baru `navigation_mode`), lalu update Rust structs agar compile, lalu SQL UPDATE/INSERT data, lalu update TypeScript admin. Quiz FE (`navigation_mode` behavior) dikerjakan di sprint terpisah — tiket ini hanya expose field-nya saja.

**Tech Stack:** MySQL 8, Rust (Actix-Web + SQLx + serde_json), Next.js 15 (TypeScript)

---

## File Map

| File | Action | Perubahan |
|---|---|---|
| `quiz-backend/src/model/paket_soal.rs:291-338` | Modify | Tambah `section_duration_minutes` ke `SimulasiTemplateSection`; `navigation_mode` ke `SimulasiTemplate` & `SimulasiTemplateRequest` |
| `quiz-backend/src/controller/admin_packages_controller.rs:1207-1310` | Modify | Update SELECT (tambah `navigation_mode`), INSERT, UPDATE SQL |
| `quiz-admin/src/lib/api.ts:573-581` | Modify | Tambah `SimulasiTemplateSection` interface; update `SimulasiTemplate` |
| `quiz-admin/src/app/admin/packages/generate-simulasi/page.tsx:22-29` | Modify | Update `MOCK_TEMPLATES` dan `EXAM_ICONS` |

---

## Task 1: ALTER TABLE di DEV (Wajib sebelum Rust compile)

**Files:** DB DEV saja (DDL)

- [ ] **Step 1: Jalankan ALTER TABLE di DEV**

```bash
sudo mysql -u root dbquizapp -e "
ALTER TABLE simulasi_templates
  ADD COLUMN navigation_mode ENUM('free', 'section_locked') NOT NULL DEFAULT 'free';
"
```

Expected output: tidak ada error, exit code 0.

- [ ] **Step 2: Verifikasi kolom sudah ada**

```bash
sudo mysql -u root dbquizapp -e "DESCRIBE simulasi_templates;" | grep navigation_mode
```

Expected:
```
navigation_mode | enum('free','section_locked') | NO | | free |
```

- [ ] **Step 3: Commit DDL ke file migration (opsional tapi dianjurkan)**

Buat file `quiz-backend/migrations/2026-05-31-add-navigation-mode.sql`:

```sql
ALTER TABLE simulasi_templates
  ADD COLUMN navigation_mode ENUM('free', 'section_locked') NOT NULL DEFAULT 'free';
```

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
git add migrations/2026-05-31-add-navigation-mode.sql
git commit -m "chore(db): add navigation_mode column to simulasi_templates"
```

---

## Task 2: Update Rust Model — SimulasiTemplateSection

**Files:**
- Modify: `quiz-backend/src/model/paket_soal.rs:291-295`

- [ ] **Step 1: Tambah `section_duration_minutes` ke struct**

Di `src/model/paket_soal.rs`, ganti baris 291–295:

```rust
// SEBELUM:
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SimulasiTemplateSection {
    pub name: String,
    pub subcategory_slug: String,
    pub count: u32,
}

// SESUDAH:
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SimulasiTemplateSection {
    pub name: String,
    pub subcategory_slug: String,
    pub count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_duration_minutes: Option<u32>,
}
```

`skip_serializing_if` memastikan field tidak muncul di JSON jika `null` — FE tidak perlu handle `null` untuk template yang tidak punya timer per-section.

---

## Task 3: Update Rust Model — SimulasiTemplate

**Files:**
- Modify: `quiz-backend/src/model/paket_soal.rs:297-327`

- [ ] **Step 1: Tambah `navigation_mode` ke struct dan FromRow**

Ganti seluruh blok `SimulasiTemplate` struct + `impl FromRow` (baris 297–327):

```rust
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SimulasiTemplate {
    pub id: i32,
    pub exam_type: String,
    pub kode_prefix: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<SimulasiTemplateSection>,
    pub duration_minutes: i32,
    pub passing_score: i32,
    pub is_active: bool,
    pub navigation_mode: String,
}

impl<'c> FromRow<'c, MySqlRow> for SimulasiTemplate {
    fn from_row(row: &'c MySqlRow) -> Result<Self, sqlx::Error> {
        let sections_json: String = row.try_get("sections").unwrap_or_else(|_| "[]".to_string());
        let sections: Vec<SimulasiTemplateSection> =
            serde_json::from_str(&sections_json).unwrap_or_default();
        Ok(SimulasiTemplate {
            id: row.get("id"),
            exam_type: row.get("exam_type"),
            kode_prefix: row.get("kode_prefix"),
            name: row.get("name"),
            description: row.try_get("description").ok(),
            sections,
            duration_minutes: row.get("duration_minutes"),
            passing_score: row.get("passing_score"),
            is_active: row.get("is_active"),
            navigation_mode: row.try_get("navigation_mode").unwrap_or_else(|_| "free".to_string()),
        })
    }
}
```

---

## Task 4: Update Rust Model — SimulasiTemplateRequest

**Files:**
- Modify: `quiz-backend/src/model/paket_soal.rs:329-338`

- [ ] **Step 1: Tambah `navigation_mode` ke request struct**

Ganti blok `SimulasiTemplateRequest` (baris 329–338):

```rust
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SimulasiTemplateRequest {
    pub exam_type: String,
    pub kode_prefix: String,
    pub name: String,
    pub description: Option<String>,
    pub sections: Vec<SimulasiTemplateSection>,
    pub duration_minutes: i32,
    pub passing_score: Option<i32>,
    pub navigation_mode: Option<String>,
}
```

---

## Task 5: Update Controller — SELECT, INSERT, UPDATE

**Files:**
- Modify: `quiz-backend/src/controller/admin_packages_controller.rs:1213-1309`

- [ ] **Step 1: Update `list_simulasi_templates` SELECT (baris ~1213-1214)**

Ganti query di `list_simulasi_templates`:

```rust
// SEBELUM:
"SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active FROM simulasi_templates ORDER BY id"

// SESUDAH:
"SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active, navigation_mode FROM simulasi_templates ORDER BY id"
```

- [ ] **Step 2: Update `create_simulasi_template` INSERT + SELECT (baris ~1241-1258)**

Ganti query INSERT dan bind-nya:

```rust
// SEBELUM INSERT:
let result = sqlx::query(
    "INSERT INTO simulasi_templates (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score) VALUES (?, ?, ?, ?, ?, ?, ?)"
)
.bind(req.exam_type.trim())
.bind(req.kode_prefix.trim().to_uppercase())
.bind(req.name.trim())
.bind(&req.description)
.bind(&sections_json)
.bind(req.duration_minutes)
.bind(req.passing_score.unwrap_or(60))
.execute(&*data.context.soal.pool)
.await;

// SESUDAH INSERT:
let result = sqlx::query(
    "INSERT INTO simulasi_templates (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
)
.bind(req.exam_type.trim())
.bind(req.kode_prefix.trim().to_uppercase())
.bind(req.name.trim())
.bind(&req.description)
.bind(&sections_json)
.bind(req.duration_minutes)
.bind(req.passing_score.unwrap_or(60))
.bind(req.navigation_mode.as_deref().unwrap_or("free"))
.execute(&*data.context.soal.pool)
.await;
```

Ganti juga SELECT setelah INSERT (baris ~1257-1258):

```rust
// SEBELUM:
"SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active FROM simulasi_templates WHERE id = ?"

// SESUDAH:
"SELECT id, exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, is_active, navigation_mode FROM simulasi_templates WHERE id = ?"
```

- [ ] **Step 3: Update `update_simulasi_template` UPDATE (baris ~1288-1299)**

Ganti query UPDATE dan bind-nya:

```rust
// SEBELUM:
let result = sqlx::query(
    "UPDATE simulasi_templates SET exam_type=?, kode_prefix=?, name=?, description=?, sections=?, duration_minutes=?, passing_score=? WHERE id=?"
)
.bind(req.exam_type.trim())
.bind(req.kode_prefix.trim().to_uppercase())
.bind(req.name.trim())
.bind(&req.description)
.bind(&sections_json)
.bind(req.duration_minutes)
.bind(req.passing_score.unwrap_or(60))
.bind(template_id)
.execute(&*data.context.soal.pool)
.await;

// SESUDAH:
let result = sqlx::query(
    "UPDATE simulasi_templates SET exam_type=?, kode_prefix=?, name=?, description=?, sections=?, duration_minutes=?, passing_score=?, navigation_mode=? WHERE id=?"
)
.bind(req.exam_type.trim())
.bind(req.kode_prefix.trim().to_uppercase())
.bind(req.name.trim())
.bind(&req.description)
.bind(&sections_json)
.bind(req.duration_minutes)
.bind(req.passing_score.unwrap_or(60))
.bind(req.navigation_mode.as_deref().unwrap_or("free"))
.bind(template_id)
.execute(&*data.context.soal.pool)
.await;
```

---

## Task 6: cargo build — Verifikasi 0 Error

**Files:** (none — build only)

- [ ] **Step 1: Build backend**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | tail -5
```

Expected output akhir: `Finished \`dev\` profile ...` tanpa `error[...]`.

- [ ] **Step 2: Commit Rust changes**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
git add src/model/paket_soal.rs src/controller/admin_packages_controller.rs
git commit -m "feat(AFD-251): add navigation_mode & section_duration_minutes to simulasi templates"
```

---

## Task 7: SQL UPDATE/INSERT Data — DEV DB

**Files:** DB DEV saja

- [ ] **Step 1: UPDATE template SKD, RBB, STAN**

```bash
sudo mysql -u root dbquizapp << 'EOF'
-- SKD CPNS: 30+35+45=110 soal, 100 menit, pg 57%
UPDATE simulasi_templates
SET
  sections = '[{"name":"TWK","subcategory_slug":"twk-tes-wawasan-kebangsaan","count":30},{"name":"TIU","subcategory_slug":"tiu-tes-intelegensi-umum","count":35},{"name":"TKP","subcategory_slug":"tkp-tes-karakteristik-pribadi","count":45}]',
  duration_minutes = 100,
  passing_score = 57,
  navigation_mode = 'free'
WHERE exam_type = 'skd';

-- RBB BUMN: TKD 40 + AKHLAK 30 + Inggris 30 = 100 soal, 90 menit
UPDATE simulasi_templates
SET
  sections = '[{"name":"TKD","subcategory_slug":"tkd-bumn","count":40},{"name":"Core Values AKHLAK","subcategory_slug":"akhlak-bumn","count":30},{"name":"Bahasa Inggris","subcategory_slug":"english-bumn","count":30}]',
  navigation_mode = 'free'
WHERE exam_type = 'rbb';

-- PKN STAN: 110 soal, 100 menit, pg 57%
UPDATE simulasi_templates
SET
  sections = '[{"name":"SKD STAN","subcategory_slug":"skd-stan","count":110}]',
  duration_minutes = 100,
  passing_score = 57,
  navigation_mode = 'free'
WHERE exam_type = 'stan';
EOF
```

- [ ] **Step 2: INSERT template PPPK (non-teknis, karena soal teknis belum ada)**

```bash
sudo mysql -u root dbquizapp << 'EOF'
INSERT INTO simulasi_templates
  (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode)
VALUES (
  'pppk', 'PPPK', 'Simulasi PPPK — Non-Teknis',
  'Kompetensi Manajerial 25 + Sosial Kultural 20 = 45 soal, 60 menit. (Kompetensi Teknis & Wawancara menyusul.)',
  '[{"name":"Kompetensi Manajerial","subcategory_slug":"pppk-manajerial","count":25},{"name":"Kompetensi Sosial Kultural","subcategory_slug":"pppk-sosiokultural","count":20}]',
  60, 0, 'section_locked'
);
EOF
```

- [ ] **Step 3: INSERT template LPDP TBS**

```bash
sudo mysql -u root dbquizapp << 'EOF'
INSERT INTO simulasi_templates
  (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode)
VALUES (
  'lpdp', 'LPDP', 'Simulasi LPDP — Tes Bakat Skolastik',
  'Verbal 23/30 mnt + Kuantitatif 25/40 mnt + Pemecahan Masalah 12/20 mnt = 60 soal, 90 menit. Timer per section, tidak bisa balik ke section sebelumnya.',
  '[{"name":"Penalaran Verbal","subcategory_slug":"penalaran-verbal","count":23,"section_duration_minutes":30},{"name":"Penalaran Kuantitatif","subcategory_slug":"penalaran-numerik","count":25,"section_duration_minutes":40},{"name":"Pemecahan Masalah","subcategory_slug":"pemecahan-masalah","count":12,"section_duration_minutes":20}]',
  90, 42, 'section_locked'
);
EOF
```

- [ ] **Step 4: Verifikasi semua template di DEV**

```bash
sudo mysql -u root dbquizapp -e "SELECT id, exam_type, navigation_mode, duration_minutes, passing_score FROM simulasi_templates ORDER BY id;"
```

Expected (5 rows):

```
+----+-----------+-----------------+------------------+--------------+
| id | exam_type | navigation_mode | duration_minutes | passing_score |
+----+-----------+-----------------+------------------+--------------+
|  1 | skd       | free            |              100 |            57 |
|  2 | rbb       | free            |               90 |            60 |
|  3 | stan      | free            |              100 |            57 |
| .. | pppk      | section_locked  |               60 |             0 |
| .. | lpdp      | section_locked  |               90 |            42 |
+----+-----------+-----------------+------------------+--------------+
```

- [ ] **Step 5: Verifikasi sections SKD benar**

```bash
sudo mysql -u root dbquizapp -e "SELECT sections FROM simulasi_templates WHERE exam_type='skd';" | python3 -m json.tool 2>/dev/null || sudo mysql -u root dbquizapp -e "SELECT sections FROM simulasi_templates WHERE exam_type='skd';"
```

Expected: `count` harus 30, 35, 45 (bukan 35, 30, 35).

---

## Task 8: Update Admin TypeScript Interface

**Files:**
- Modify: `quiz-admin/src/lib/api.ts:571-581`

- [ ] **Step 1: Tambah `SimulasiTemplateSection` dan update `SimulasiTemplate`**

Ganti blok di `src/lib/api.ts` (baris 571–581):

```typescript
// ── AFD-243: Simulasi templates & batch generate ──────────────────────────────

export interface SimulasiTemplateSection {
  name: string
  subcategory_slug: string
  count: number
  section_duration_minutes?: number
}

export interface SimulasiTemplate {
  id: number
  exam_type: string
  kode_prefix: string
  name: string
  description?: string
  duration_minutes: number
  passing_score: number
  is_active: boolean
  navigation_mode: string
  sections: SimulasiTemplateSection[]
}
```

---

## Task 9: Update Admin MOCK_TEMPLATES

**Files:**
- Modify: `quiz-admin/src/app/admin/packages/generate-simulasi/page.tsx:22-45`

- [ ] **Step 1: Update MOCK_TEMPLATES dan EXAM_ICONS**

Ganti blok `MOCK_TEMPLATES` dan `EXAM_ICONS` (baris 22–45):

```typescript
// ── Mock data (remove when API selalu return data) ────────────────────────────

const MOCK_TEMPLATES: SimulasiTemplate[] = [
  {
    id: 1, exam_type: 'skd', kode_prefix: 'SKD', name: 'SKD CPNS',
    description: '30 TWK + 35 TIU + 45 TKP = 110 soal, 100 menit',
    duration_minutes: 100, passing_score: 57, navigation_mode: 'free', is_active: true,
    sections: [
      { name: 'TWK', subcategory_slug: 'twk-tes-wawasan-kebangsaan', count: 30 },
      { name: 'TIU', subcategory_slug: 'tiu-tes-intelegensi-umum', count: 35 },
      { name: 'TKP', subcategory_slug: 'tkp-tes-karakteristik-pribadi', count: 45 },
    ],
  },
  {
    id: 2, exam_type: 'rbb', kode_prefix: 'RBB', name: 'RBB BUMN',
    description: 'TKD 40 + AKHLAK 30 + Bahasa Inggris 30 = 100 soal, 90 menit',
    duration_minutes: 90, passing_score: 60, navigation_mode: 'free', is_active: true,
    sections: [
      { name: 'TKD', subcategory_slug: 'tkd-bumn', count: 40 },
      { name: 'Core Values AKHLAK', subcategory_slug: 'akhlak-bumn', count: 30 },
      { name: 'Bahasa Inggris', subcategory_slug: 'english-bumn', count: 30 },
    ],
  },
  {
    id: 3, exam_type: 'stan', kode_prefix: 'STAN', name: 'PKN STAN',
    description: 'SKD 110 soal, 100 menit',
    duration_minutes: 100, passing_score: 57, navigation_mode: 'free', is_active: true,
    sections: [
      { name: 'SKD STAN', subcategory_slug: 'skd-stan', count: 110 },
    ],
  },
  {
    id: 4, exam_type: 'pppk', kode_prefix: 'PPPK', name: 'Simulasi PPPK — Non-Teknis',
    description: 'Manajerial 25 + Sosial Kultural 20 = 45 soal, 60 menit',
    duration_minutes: 60, passing_score: 0, navigation_mode: 'section_locked', is_active: true,
    sections: [
      { name: 'Kompetensi Manajerial', subcategory_slug: 'pppk-manajerial', count: 25 },
      { name: 'Kompetensi Sosial Kultural', subcategory_slug: 'pppk-sosiokultural', count: 20 },
    ],
  },
  {
    id: 5, exam_type: 'lpdp', kode_prefix: 'LPDP', name: 'Simulasi LPDP — TBS',
    description: 'Verbal 23/30 mnt + Kuantitatif 25/40 mnt + Pemecahan 12/20 mnt = 60 soal',
    duration_minutes: 90, passing_score: 42, navigation_mode: 'section_locked', is_active: true,
    sections: [
      { name: 'Penalaran Verbal', subcategory_slug: 'penalaran-verbal', count: 23, section_duration_minutes: 30 },
      { name: 'Penalaran Kuantitatif', subcategory_slug: 'penalaran-numerik', count: 25, section_duration_minutes: 40 },
      { name: 'Pemecahan Masalah', subcategory_slug: 'pemecahan-masalah', count: 12, section_duration_minutes: 20 },
    ],
  },
]

const KNOWN_SOURCES = ['jadiasn', 'tryoutsiswa', 'jadibeasiswa', 'bangshoal', 'zenius', 'cpnsonline']

const EXAM_ICONS: Record<string, string> = {
  skd: '🏛️', rbb: '🏢', stan: '🎓', pppk: '📚', lpdp: '🎯',
}
```

---

## Task 10: Admin Build & Commit

**Files:** (none — build only)

- [ ] **Step 1: Build admin frontend**

```bash
cd /mnt/devarea/devarea/MONOREPO_QUIZ/quiz-admin
npm run build 2>&1 | tail -10
```

Expected: `✓ Compiled successfully` tanpa TypeScript error.

- [ ] **Step 2: Commit admin changes**

```bash
cd /mnt/devarea/devarea/MONOREPO_QUIZ/quiz-admin
git add src/lib/api.ts src/app/admin/packages/generate-simulasi/page.tsx
git commit -m "feat(AFD-251): update SimulasiTemplate interface & MOCK_TEMPLATES — navigation_mode, sections, PPPK, LPDP"
```

---

## Task 11: Deploy ke PROD

**Files:** PROD DB + remote push

> ⚠️ Lakukan task ini hanya setelah Task 1–10 selesai dan DEV sudah verified.

- [ ] **Step 1: ALTER TABLE di PROD**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp -e "
ALTER TABLE simulasi_templates
  ADD COLUMN navigation_mode ENUM('free', 'section_locked') NOT NULL DEFAULT 'free';
"
```

- [ ] **Step 2: Verifikasi kolom di PROD**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp -e "DESCRIBE simulasi_templates;" | grep navigation_mode
```

- [ ] **Step 3: Push BE ke prod**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
git push prod main
```

Tunggu build/restart selesai (cek log jika perlu).

- [ ] **Step 4: SQL UPDATE/INSERT di PROD**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp << 'EOF'
-- SKD
UPDATE simulasi_templates
SET
  sections = '[{"name":"TWK","subcategory_slug":"twk-tes-wawasan-kebangsaan","count":30},{"name":"TIU","subcategory_slug":"tiu-tes-intelegensi-umum","count":35},{"name":"TKP","subcategory_slug":"tkp-tes-karakteristik-pribadi","count":45}]',
  duration_minutes = 100,
  passing_score = 57,
  navigation_mode = 'free'
WHERE exam_type = 'skd';

-- RBB
UPDATE simulasi_templates
SET
  sections = '[{"name":"TKD","subcategory_slug":"tkd-bumn","count":40},{"name":"Core Values AKHLAK","subcategory_slug":"akhlak-bumn","count":30},{"name":"Bahasa Inggris","subcategory_slug":"english-bumn","count":30}]',
  navigation_mode = 'free'
WHERE exam_type = 'rbb';

-- STAN
UPDATE simulasi_templates
SET
  sections = '[{"name":"SKD STAN","subcategory_slug":"skd-stan","count":110}]',
  duration_minutes = 100,
  passing_score = 57,
  navigation_mode = 'free'
WHERE exam_type = 'stan';

-- PPPK (non-teknis)
INSERT INTO simulasi_templates
  (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode)
VALUES (
  'pppk', 'PPPK', 'Simulasi PPPK — Non-Teknis',
  'Kompetensi Manajerial 25 + Sosial Kultural 20 = 45 soal, 60 menit. (Kompetensi Teknis & Wawancara menyusul.)',
  '[{"name":"Kompetensi Manajerial","subcategory_slug":"pppk-manajerial","count":25},{"name":"Kompetensi Sosial Kultural","subcategory_slug":"pppk-sosiokultural","count":20}]',
  60, 0, 'section_locked'
);

-- LPDP TBS
INSERT INTO simulasi_templates
  (exam_type, kode_prefix, name, description, sections, duration_minutes, passing_score, navigation_mode)
VALUES (
  'lpdp', 'LPDP', 'Simulasi LPDP — Tes Bakat Skolastik',
  'Verbal 23/30 mnt + Kuantitatif 25/40 mnt + Pemecahan Masalah 12/20 mnt = 60 soal, 90 menit. Timer per section, tidak bisa balik ke section sebelumnya.',
  '[{"name":"Penalaran Verbal","subcategory_slug":"penalaran-verbal","count":23,"section_duration_minutes":30},{"name":"Penalaran Kuantitatif","subcategory_slug":"penalaran-numerik","count":25,"section_duration_minutes":40},{"name":"Pemecahan Masalah","subcategory_slug":"pemecahan-masalah","count":12,"section_duration_minutes":20}]',
  90, 42, 'section_locked'
);
EOF
```

- [ ] **Step 5: Verifikasi API PROD**

```bash
curl -s https://<prod-domain>/admin/packages/simulasi-templates | python3 -m json.tool | grep -E '"exam_type"|"navigation_mode"|"duration_minutes"'
```

Expected: 5 templates, SKD punya `navigation_mode: "free"`, LPDP punya `navigation_mode: "section_locked"`.

- [ ] **Step 6: Push admin FE ke prod**

```bash
cd /mnt/devarea/devarea/MONOREPO_QUIZ/quiz-admin
git push prod main
```

- [ ] **Step 7: Checklist verifikasi akhir**

- [ ] `GET /admin/packages/simulasi-templates` mengembalikan 5 template dengan field `navigation_mode` dan `sections`
- [ ] SKD: `count` = 30+35+45, `duration_minutes=100`, `passing_score=57`
- [ ] Generate SKD 1 paket berhasil (bukan 400 error)
- [ ] Paket yang digenerate punya 110 soal di `paket_soal_items`
- [ ] LPDP: `sections[0].section_duration_minutes = 30` terbaca di response
- [ ] PPPK: generate berhasil 45 soal (Manajerial+Sosiokultural)
- [ ] Admin UI: template cards tampil data terbaru (bukan mock lama)
- [ ] Result simulasi SKD: user dengan skor ≥57% tampil LULUS
