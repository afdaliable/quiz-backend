# AFD-252: TKP Scoring Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix TKP (Tes Karakteristik Pribadi) scoring so each option awards its correct 1-5 poin instead of binary correct/wrong, by adding `option_scores` JSON column, backfilling from solution HTML, updating backend scoring, and updating frontend display.

**Architecture:** DB migration adds `option_scores JSON` column and updates `question_type='tkp'` for TKP soal. A Python backfill script parses 3 known HTML formats from `solution` field. Backend scoring in `quiz_session_dao.rs` is updated to use `option_scores` for TKP soal. Frontend passes `option_scores` through API response and renders TKP differently (no binary green/red, shows poin badge in review mode).

**Tech Stack:** Rust/SQLx (backend), MySQL JSON column, Python 3 (migration script), Angular 17 (frontend)

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `quiz-backend/src/dao/quiz_session_dao.rs` | Modify | Core scoring logic — `calculate_score_by_ids`, `calculate_score` |
| `quiz-backend/src/model/soal.rs` | Modify | Add `option_scores` to `Soal` struct |
| `quiz-backend/src/dao/db_context.rs` | Modify | `get_paket_soal_response` — expose `option_scores` in soal response |
| `quiz-backend/src/model/paket_soal_response.rs` (check) | Modify if needed | `PaketSoalResponse` struct — add `option_scores` field |
| `quiz-backend/scripts/backfill_tkp_scores.py` | Create | One-time migration: parse solution HTML → populate `option_scores` |
| `quiz-frontend/src/app/services/question.service.ts` | Modify | Map `option_scores` from API response |
| `quiz-frontend/src/app/question/question.component.ts` | Modify | TKP-aware `calculateScore()`, study mode feedback |
| `quiz-frontend/src/app/question/question.component.html` | Modify | Badge poin per opsi in review/study mode |

---

## Task 1: DB Schema — Add `option_scores` column

**Files:**
- Run SQL directly on PROD DB (no ORM migration file needed — SQLx uses raw queries)

- [ ] **Step 1: Add column**

```sql
ALTER TABLE soal
ADD COLUMN option_scores JSON NULL
    COMMENT 'TKP scores per option: {"opt1":2,"opt2":3,"opt3":5,"opt4":1,"opt5":4}';
```

Run: `mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp -e "ALTER TABLE soal ADD COLUMN option_scores JSON NULL COMMENT 'TKP scores per option: {\"opt1\":2,\"opt2\":3,\"opt3\":5,\"opt4\":1,\"opt5\":4}';" 2>/dev/null`

Expected: Query OK

- [ ] **Step 2: Verify column added**

```sql
SHOW COLUMNS FROM soal LIKE 'option_scores';
```

Expected output: `option_scores | longtext | YES | | NULL |`

- [ ] **Step 3: Commit note (no code commit for schema — just checkpoint)**

Document: column added, all rows currently NULL.

---

## Task 2: Migration Script — Backfill `option_scores` from solution HTML

**Files:**
- Create: `quiz-backend/scripts/backfill_tkp_scores.py`

- [ ] **Step 1: Create the script**

```python
#!/usr/bin/env python3
"""
AFD-252: Backfill option_scores for TKP soal.
Run once: python3 scripts/backfill_tkp_scores.py
Reads solution HTML, parses 3 known formats, writes JSON to option_scores.
"""
import re
import json
import mysql.connector

# --- Config ---
DB = dict(
    host='100.87.162.99',
    user='root',
    password='love4JJI#123somuch',
    database='dbquizapp',
)

LETTER_TO_OPT = {'A': 'opt1', 'B': 'opt2', 'C': 'opt3', 'D': 'opt4', 'E': 'opt5'}

# --- Parsers ---

def parse_fmt1(solution: str):
    """Format: 'A: 2 poin'  (jadiasn — 1342 soal)"""
    m = re.findall(r'([A-E]):\s*([1-5])\s*poin', solution)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def parse_fmt2(solution: str):
    """Format: 'Opsi A 2 poin'  (alfaiz — 2114 soal)"""
    m = re.findall(r'Opsi\s+([A-E])\s+([1-5])\s+poin', solution)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def parse_fmt3(solution: str):
    """Format: 'A. teks - Nilai (2)'  (belajarbro — ~1044 soal)"""
    # Use re.DOTALL in case option text spans lines
    m = re.findall(r'([A-E])\.\s+.*?-\s*Nilai\s*\(([1-5])\)', solution, re.DOTALL)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def default_scores(correct_answer: str) -> dict:
    """
    Fallback for 45 cpnsonline soal with no solution.
    correct_answer gets 5, others get 4,3,2,1 in order.
    """
    opts = ['opt1', 'opt2', 'opt3', 'opt4', 'opt5']
    others = [o for o in opts if o != correct_answer]
    result = {correct_answer: 5}
    for i, opt in enumerate(others):
        result[opt] = 4 - i
    return result


def parse_scores(solution, correct_answer):
    if solution:
        return (
            parse_fmt1(solution)
            or parse_fmt2(solution)
            or parse_fmt3(solution)
        )
    return default_scores(correct_answer)


# --- Main ---

def main():
    cn = mysql.connector.connect(**DB)
    cur = cn.cursor(dictionary=True)

    # Fetch all TKP soal
    cur.execute("""
        SELECT s.id, s.solution, s.correct_answer
        FROM soal s
        WHERE s.subcategory_id IN (
            SELECT id FROM subcategories WHERE slug = 'tkp-tes-karakteristik-pribadi'
        )
        AND s.status = 'active'
        AND s.option_scores IS NULL
    """)
    rows = cur.fetchall()
    print(f"Found {len(rows)} TKP soal to backfill")

    ok = skip = fail = 0

    update_cur = cn.cursor()
    for row in rows:
        scores = parse_scores(row['solution'] or '', row['correct_answer'] or 'opt1')
        if scores and len(scores) == 5:
            update_cur.execute(
                "UPDATE soal SET option_scores = %s WHERE id = %s",
                (json.dumps(scores), row['id'])
            )
            ok += 1
        elif not row['solution']:
            # Default scores for no-solution soal
            scores = default_scores(row['correct_answer'] or 'opt1')
            update_cur.execute(
                "UPDATE soal SET option_scores = %s WHERE id = %s",
                (json.dumps(scores), row['id'])
            )
            skip += 1
        else:
            print(f"  FAIL to parse id={row['id']}: {str(row['solution'])[:100]}")
            fail += 1

    cn.commit()
    print(f"\nDone. ok={ok}  default_fallback={skip}  failed={fail}")
    print(f"Total updated: {ok + skip} / {len(rows)}")

    # Set question_type = 'tkp' for all successfully backfilled soal
    update_cur.execute("""
        UPDATE soal
        SET question_type = 'tkp'
        WHERE subcategory_id IN (
            SELECT id FROM subcategories WHERE slug = 'tkp-tes-karakteristik-pribadi'
        )
        AND status = 'active'
        AND option_scores IS NOT NULL
    """)
    cn.commit()
    print(f"Updated question_type='tkp' for {update_cur.rowcount} soal")

    cur.close()
    update_cur.close()
    cn.close()


if __name__ == '__main__':
    main()
```

- [ ] **Step 2: Run dry-run check first (count only)**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp 2>/dev/null <<'EOF'
SELECT COUNT(*) AS tkp_soal_to_fill
FROM soal
WHERE subcategory_id IN (SELECT id FROM subcategories WHERE slug='tkp-tes-karakteristik-pribadi')
AND status='active' AND option_scores IS NULL;
EOF
```

Expected: `4545`

- [ ] **Step 3: Run migration script**

```bash
pip3 install mysql-connector-python -q
cd /mnt/devarea/devarea/rustproject/quiz-backend
python3 scripts/backfill_tkp_scores.py
```

Expected output:
```
Found 4545 TKP soal to backfill
Done. ok=4500  default_fallback=45  failed=0
Total updated: 4545 / 4545
Updated question_type='tkp' for 4545 soal
```

- [ ] **Step 4: Verify migration result**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp 2>/dev/null <<'EOF'
SELECT 
    COUNT(*) AS total,
    SUM(option_scores IS NOT NULL) AS has_scores,
    SUM(question_type = 'tkp') AS is_tkp_type,
    SUM(option_scores IS NULL) AS missing
FROM soal
WHERE subcategory_id IN (SELECT id FROM subcategories WHERE slug='tkp-tes-karakteristik-pribadi')
AND status='active';
EOF
```

Expected: `total=4545, has_scores=4545, is_tkp_type=4545, missing=0`

- [ ] **Step 5: Spot-check a row**

```bash
mysql -h 100.87.162.99 -u root -p'love4JJI#123somuch' dbquizapp 2>/dev/null <<'EOF'
SELECT id, question_type, correct_answer, option_scores
FROM soal WHERE id = 17998;
EOF
```

Expected: `question_type=tkp`, `correct_answer=opt3`, `option_scores={"opt1":2,"opt2":3,"opt3":5,"opt4":1,"opt5":4}`

- [ ] **Step 6: Commit script**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
git add scripts/backfill_tkp_scores.py
git commit -m "feat(tkp): add backfill migration script for option_scores

Parses 3 solution HTML formats (A: N poin, Opsi A N poin, A. - Nilai(N))
from 4545 TKP soal and populates option_scores JSON column.
Also sets question_type='tkp' for all backfilled soal."
```

---

## Task 3: Backend — Add `option_scores` to Soal model

**Files:**
- Modify: `quiz-backend/src/model/soal.rs`

- [ ] **Step 1: Read current Soal struct**

Current relevant fields in `src/model/soal.rs` (around line 10-40):
```rust
pub struct Soal {
    pub id: i32,
    // ... other fields ...
    pub question_type: String,
    // ... options ...
    pub correct_answer: Option<String>,
    // NO option_scores field yet
}
```

- [ ] **Step 2: Add `option_scores` to `Soal` struct**

In `src/model/soal.rs`, find the `Soal` struct and add after `correct_answer`:

```rust
pub correct_answer: Option<String>,
pub option_scores: Option<serde_json::Value>,  // TKP: {"opt1":2,"opt2":3,"opt3":5,"opt4":1,"opt5":4}
```

- [ ] **Step 3: Add to `from_row` mapping in `Soal` impl**

Find where `Soal` is constructed from a DB row (around line 115-130 in `soal.rs`). After `correct_answer`:

```rust
correct_answer: row.get("correct_answer"),
option_scores: row.try_get::<Option<serde_json::Value>, _>("option_scores").unwrap_or(None),
```

- [ ] **Step 4: Also add to `AdminSoal` struct** (around line 152)

Same pattern — add after `correct_answer`:
```rust
pub correct_answer: Option<String>,
pub option_scores: Option<serde_json::Value>,
```

And in its row mapping (around line 226):
```rust
correct_answer: row.get("correct_answer"),
option_scores: row.try_get::<Option<serde_json::Value>, _>("option_scores").unwrap_or(None),
```

- [ ] **Step 5: Build to verify no compile errors**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | grep "^error" | head -20
```

Expected: No errors (only warnings OK)

- [ ] **Step 6: Commit**

```bash
git add src/model/soal.rs
git commit -m "feat(tkp): add option_scores field to Soal and AdminSoal structs"
```

---

## Task 4: Backend — Update scoring to handle TKP

**Files:**
- Modify: `quiz-backend/src/dao/quiz_session_dao.rs` — `calculate_score_by_ids` and `calculate_score`

### 4A — Update `calculate_score_by_ids` (used by random + simulasi sessions)

- [ ] **Step 1: Update SQL to fetch `question_type` and `option_scores`**

In `calculate_score_by_ids` (line ~607), change:
```rust
// OLD:
"SELECT id, correct_answer FROM soal WHERE id IN ({})",
// NEW:
"SELECT id, correct_answer, question_type, option_scores FROM soal WHERE id IN ({})",
```

- [ ] **Step 2: Update the row mapping to capture new fields**

Replace the map (old: only `correct_answer`) with:

```rust
// Build map id → (correct_answer, question_type, option_scores)
let soal_map: HashMap<i32, (String, String, Option<serde_json::Value>)> = rows
    .into_iter()
    .filter_map(|row| {
        let id: i32 = row.get("id");
        let ca: Option<String> = row.get("correct_answer");
        let qt: String = row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string());
        let os: Option<serde_json::Value> = row.try_get("option_scores").unwrap_or(None);
        ca.map(|c| (id, (c, qt, os)))
    })
    .collect();
```

- [ ] **Step 3: Replace the scoring loop with TKP-aware logic**

Replace the current loop (line ~626-658) with:

```rust
let mut raw_score = 0i32;       // actual points earned
let mut max_score = 0i32;       // max possible points
let mut correct_count = 0i32;   // soal with max poin (5)
let mut incorrect_count = 0i32; // TWK/TIU wrong; TKP unanswered

for (index, qid) in question_ids.iter().enumerate() {
    let user_answer_idx: Option<i32> = user_answers
        .get(index)
        .and_then(|a| *a);

    if let Some((correct_answer, question_type, option_scores)) = soal_map.get(qid) {
        max_score += 5;

        let Some(ans_idx) = user_answer_idx else {
            // Unanswered
            if question_type != "tkp" {
                // TWK/TIU: unanswered = 0, no count
            }
            continue;
        };

        // Convert index (0-4) to "opt1"-"opt5"
        let opt_key = match ans_idx {
            0 => "opt1", 1 => "opt2", 2 => "opt3", 3 => "opt4", 4 => "opt5",
            _ => continue,
        };

        if question_type == "tkp" {
            // TKP: look up poin from option_scores, minimum 1
            let poin = option_scores
                .as_ref()
                .and_then(|os| os.get(opt_key))
                .and_then(|v| v.as_i64())
                .unwrap_or(1) as i32;
            raw_score += poin;
            if poin == 5 {
                correct_count += 1;
            }
        } else {
            // TWK / TIU: binary 5 or 0
            if opt_key == correct_answer.as_str() {
                raw_score += 5;
                correct_count += 1;
            } else {
                incorrect_count += 1;
            }
        }
    }
}

let score = if max_score > 0 {
    (raw_score as f32 / max_score as f32 * 100.0).round() as i32
} else {
    0
};

Ok((correct_count, incorrect_count, score))
```

### 4B — Update `calculate_score` (used by standard paket sessions)

- [ ] **Step 4: Update SQL in `calculate_score` to fetch `question_type` and `option_scores`**

In `calculate_score` (line ~667), change:
```rust
// OLD:
"SELECT s.correct_answer
 FROM kategori_soal ks
 JOIN paket_soal ps ON ks.id = ps.kategori_id
 JOIN paket_soal_items psi ON psi.paket_soal_id = ps.id
 JOIN soal s ON psi.soal_id = s.id
 WHERE ks.nama_kategori = ? AND ps.nama_paket_soal = ?
 ORDER BY psi.id"

// NEW:
"SELECT s.correct_answer, s.question_type, s.option_scores
 FROM kategori_soal ks
 JOIN paket_soal ps ON ks.id = ps.kategori_id
 JOIN paket_soal_items psi ON psi.paket_soal_id = ps.id
 JOIN soal s ON psi.soal_id = s.id
 WHERE ks.nama_kategori = ? AND ps.nama_paket_soal = ?
 ORDER BY psi.id"
```

- [ ] **Step 5: Replace the scoring loop in `calculate_score`**

Replace old loop (line ~683-715) with the same TKP-aware pattern:

```rust
let mut raw_score = 0i32;
let mut max_score = 0i32;
let mut correct_count = 0i32;
let mut incorrect_count = 0i32;

for (index, row) in paket_response.iter().enumerate() {
    let correct_answer: String = row.get("correct_answer");
    let question_type: String = row.try_get("question_type")
        .unwrap_or_else(|_| "multiple_choice".to_string());
    let option_scores: Option<serde_json::Value> = row.try_get("option_scores").unwrap_or(None);

    max_score += 5;

    let Some(user_answer_idx) = user_answers.get(index).and_then(|a| *a) else {
        continue;
    };

    let opt_key = match user_answer_idx {
        0 => "opt1", 1 => "opt2", 2 => "opt3", 3 => "opt4", 4 => "opt5",
        _ => continue,
    };

    if question_type == "tkp" {
        let poin = option_scores
            .as_ref()
            .and_then(|os| os.get(opt_key))
            .and_then(|v| v.as_i64())
            .unwrap_or(1) as i32;
        raw_score += poin;
        if poin == 5 { correct_count += 1; }
    } else {
        if opt_key == correct_answer.as_str() {
            raw_score += 5;
            correct_count += 1;
        } else {
            incorrect_count += 1;
        }
    }
}

let total_questions = paket_response.len() as i32;
let score = if total_questions > 0 {
    (raw_score as f32 / (total_questions * 5) as f32 * 100.0).round() as i32
} else {
    0
};

Ok((correct_count, incorrect_count, score))
```

- [ ] **Step 6: Build**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | grep "^error" | head -20
```

Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add src/dao/quiz_session_dao.rs
git commit -m "fix(tkp): update calculate_score to use option_scores for TKP questions

TKP soal now contribute their 1-5 poin to raw_score instead of binary
correct/wrong. Score remains percentage (raw/max*100) for backward compat.
correct_answers = soal with 5 poin (best answer for any type).
Affects both calculate_score_by_ids (random/simulasi) and calculate_score (standard)."
```

---

## Task 5: Backend — Expose `option_scores` in paket soal response

**Files:**
- Modify: `quiz-backend/src/dao/db_context.rs` — `get_paket_soal_response`
- Modify: `quiz-backend/src/model/paket_soal_response.rs` (or wherever `PaketSoalResponse` fields live)

- [ ] **Step 1: Find where individual soal fields are mapped in `get_paket_soal_response`**

```bash
grep -n "soal_id\|opt1\|correct_answer\|question_type\|kumpulan_soal" \
  /mnt/devarea/devarea/rustproject/quiz-backend/src/dao/db_context.rs | head -20
grep -n "option_scores\|question_type\|correct_answer\|struct.*Soal\|kumpulan_soal" \
  /mnt/devarea/devarea/rustproject/quiz-backend/src/model/paket_soal_response.rs 2>/dev/null | head -20
```

- [ ] **Step 2: Add `option_scores` and `question_type` to the response SQL in `get_paket_soal_response`**

In `src/dao/db_context.rs`, the SELECT query for paket soal response (around line 126-135) already selects `s.opt1` through `s.opt5`, `s.correct_answer`. Add:

```sql
-- Existing fields already include: s.id, s.soal, s.opt1-5, s.correct_answer, s.solution, s.sumberfile, s.modul, s.pelajaran, s.tag
-- ADD to SELECT:
s.question_type, s.option_scores
```

The full SELECT should become:
```sql
SELECT ks.id as kategori_id, ks.nama_kategori, ps.id as paket_soal_id, ps.nama_paket_soal, ps.is_premium,
       s.id as soal_id, s.soal, s.opt1, s.opt2, s.opt3, s.opt4, s.opt5, s.correct_answer,
       s.solution, s.sumberfile, s.modul, s.pelajaran, s.tag,
       s.question_type, s.option_scores
```

- [ ] **Step 3: Add `option_scores` and `question_type` to `PaketSoalResponse` or inner soal struct**

Find the struct that holds individual soal data in the paket response. Add:

```rust
pub question_type: String,
pub option_scores: Option<serde_json::Value>,
```

And in row mapping:
```rust
question_type: row.try_get("question_type").unwrap_or_else(|_| "multiple_choice".to_string()),
option_scores: row.try_get::<Option<serde_json::Value>, _>("option_scores").unwrap_or(None),
```

- [ ] **Step 4: Build**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | grep "^error" | head -20
```

- [ ] **Step 5: Also update `get_paket_soal_by_category` with same fields** (same file, ~line 178)

Same SQL and struct changes as Step 2-3.

- [ ] **Step 6: Also update `soal_controller.rs` — `list_soal` endpoint** to include these fields in response if not already present. Check:

```bash
grep -n "option_scores\|question_type" \
  /mnt/devarea/devarea/rustproject/quiz-backend/src/controller/soal_controller.rs | head -10
```

If not present, not needed for this ticket — users access soal via paket response.

- [ ] **Step 7: Commit**

```bash
git add src/dao/db_context.rs src/model/paket_soal_response.rs
git commit -m "feat(tkp): expose option_scores and question_type in paket soal response

Frontend needs these fields to render TKP correctly (badge poin per opsi)
and to skip binary correct/wrong feedback for TKP questions."
```

---

## Task 6: Backend — simulasi `start_simulasi` also passes `option_scores`

**Files:**
- Modify: `quiz-backend/src/controller/simulasi_ujian_controller.rs` (around line 104-136)

- [ ] **Step 1: Update the soal fetch in `start_simulasi`**

The fetch query (line ~104) already selects `id, soal, question_type, opt1-5, solution, modul, pelajaran, tag`. Add `option_scores`:

```rust
// OLD:
"SELECT id, soal, question_type, opt1, opt2, opt3, opt4, opt5, solution, modul, pelajaran, tag \
 FROM soal WHERE id IN ({})"

// NEW:
"SELECT id, soal, question_type, opt1, opt2, opt3, opt4, opt5, solution, modul, pelajaran, tag, option_scores \
 FROM soal WHERE id IN ({})"
```

- [ ] **Step 2: Add `option_scores` to the JSON response per soal** (line ~122)

```rust
soal_map.insert(id, json!({
    "id":            id,
    "soal":          row.try_get::<String, _>("soal").ok(),
    "question_type": row.try_get::<String, _>("question_type").ok().unwrap_or_else(|| "multiple_choice".to_string()),
    "opt1":          row.try_get::<Option<String>, _>("opt1").ok().flatten(),
    "opt2":          row.try_get::<Option<String>, _>("opt2").ok().flatten(),
    "opt3":          row.try_get::<Option<String>, _>("opt3").ok().flatten(),
    "opt4":          row.try_get::<Option<String>, _>("opt4").ok().flatten(),
    "opt5":          row.try_get::<Option<String>, _>("opt5").ok().flatten(),
    "solution":      row.try_get::<Option<String>, _>("solution").ok().flatten(),
    "modul":         row.try_get::<Option<String>, _>("modul").ok().flatten(),
    "pelajaran":     row.try_get::<Option<String>, _>("pelajaran").ok().flatten(),
    "tag":           row.try_get::<Option<String>, _>("tag").ok().flatten(),
    "option_scores": row.try_get::<Option<serde_json::Value>, _>("option_scores").ok().flatten(),  // ADD
}));
```

- [ ] **Step 3: Build + commit**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | grep "^error" | head -10
git add src/controller/simulasi_ujian_controller.rs
git commit -m "feat(tkp): add option_scores to simulasi start_simulasi response"
```

---

## Task 7: Backend — Build, test, push to prod

- [ ] **Step 1: Full build**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
cargo build 2>&1 | tail -5
```

Expected: `Finished dev profile`

- [ ] **Step 2: Quick sanity check — simulate SKD scoring logic mentally**

For a session with 3 soal: TWK (correct), TIU (wrong), TKP (user picks 4-poin option):
- TWK: raw+=5, correct_count+=1
- TIU: raw+=0, incorrect_count+=1
- TKP: raw+=4, (not correct since 4≠5, not incorrect)
- max = 3×5 = 15
- score = round(9/15 × 100) = 60

Old score would have been: 1 correct / 3 total = 33%. New is fairer.

- [ ] **Step 3: Push**

```bash
cd /mnt/devarea/devarea/rustproject/quiz-backend
git push prod main
```

---

## Task 8: Frontend — Map `option_scores` in `question.service.ts`

**Files:**
- Modify: `quiz-frontend/src/app/services/question.service.ts` (lines 121-141)

- [ ] **Step 1: Update `getQuestions` transform to include `option_scores`**

In the `map` block of `getQuestions` (line ~121-141), update the returned object:

```typescript
// OLD return object:
return {
  id: q.id,
  questionText: q.soal,
  question_type: questionType,
  options,
  solution: q.solution,
};

// NEW return object:
return {
  id: q.id,
  questionText: q.soal,
  question_type: questionType,
  options,
  option_scores: q.option_scores || null,  // e.g. {opt1:2,opt2:3,opt3:5,opt4:1,opt5:4}
  solution: q.solution,
};
```

- [ ] **Step 2: Also update `loadRandomQuestions` in `question.component.ts` (line ~217)**

```typescript
// OLD:
this.questionList = questions.map((q: any) => ({
  id: q.id,
  question: q.soal,
  options: [q.opt1, q.opt2, q.opt3, q.opt4, q.opt5]
    .filter((o: any) => !!o)
    .map((text: string) => ({ text, correct: false })),
  explanation: q.solution || '',
}));

// NEW:
this.questionList = questions.map((q: any) => {
  const optKeys = ['opt1', 'opt2', 'opt3', 'opt4', 'opt5'];
  const opts = optKeys
    .filter(k => q[k] != null && String(q[k]).trim() !== '')
    .map((k, i) => ({
      text: q[k],
      correct: q.correct_answer === k,
      tkp_score: q.option_scores ? (q.option_scores[k] || null) : null,
    }));
  return {
    id: q.id,
    question: q.soal,
    question_type: q.question_type || 'multiple_choice',
    options: opts,
    option_scores: q.option_scores || null,
    explanation: q.solution || '',
  };
});
```

- [ ] **Step 3: Build frontend**

```bash
cd /mnt/devarea/devarea/quiz-frontend
npm run build:prod 2>&1 | grep -E "ERROR|error TS|Output" | head -10
```

Expected: `Output location: .../dist/quiz-frontend`

- [ ] **Step 4: Commit**

```bash
cd /mnt/devarea/devarea/quiz-frontend
git add src/app/services/question.service.ts src/app/question/question.component.ts
git commit -m "feat(tkp): pass option_scores through question service and loadRandomQuestions"
```

---

## Task 9: Frontend — TKP-aware scoring in `calculateScore()`

**Files:**
- Modify: `quiz-frontend/src/app/question/question.component.ts` — `calculateScore()` (line ~608)

- [ ] **Step 1: Update `calculateScore` to handle TKP**

Replace current `calculateScore()` (line ~608-631):

```typescript
calculateScore() {
  this.correctAnswer = 0;
  this.incorrectAnswer = 0;
  let rawScore = 0;
  let maxScore = 0;
  const wrongNumbers: number[] = [];

  this.questionList.forEach((question: any, index: number) => {
    const selectedAnswer = this.selectedAnswers[index];
    const isTkp = question.question_type === 'tkp';
    maxScore += 5;

    if (selectedAnswer !== null && selectedAnswer !== undefined) {
      if (isTkp) {
        // TKP: look up poin from option_scores
        const optKey = `opt${selectedAnswer + 1}`;
        const poin = question.option_scores?.[optKey] ?? 1;
        rawScore += poin;
        if (poin === 5) this.correctAnswer++;
      } else {
        // TWK / TIU: binary
        if (question.options[selectedAnswer]?.correct) {
          rawScore += 5;
          this.correctAnswer++;
        } else {
          this.incorrectAnswer++;
          wrongNumbers.push(index + 1);
        }
      }
    }
    // unanswered: rawScore += 0
  });

  this.points = maxScore > 0
    ? Math.round((rawScore / maxScore) * 100)
    : 0;

  localStorage.setItem('wrongQuestions', JSON.stringify(wrongNumbers));
}
```

- [ ] **Step 2: Update `selectAnswer` study-mode feedback to skip TKP binary feedback**

In `selectAnswer` (around line 593-605), wrap the correct-answer reveal in a non-TKP check:

```typescript
// In study mode, show immediate full feedback (correct answer + explanation auto-open)
if (this.quizMode === 'study' && currentQno === this.currentQuestion) {
  const currentQuestionObj = this.questionList[currentQno];
  const isTkp = currentQuestionObj.question_type === 'tkp';

  if (isTkp) {
    // TKP: show poin earned, no red/green
    const optKey = `opt${option + 1}`;
    const poin = currentQuestionObj.option_scores?.[optKey] ?? 1;
    this.currentAnswerIsCorrect = poin === 5;  // show green only for perfect answer
    this.correctAnswerIndex = currentQuestionObj.options.findIndex((o: any) => o.tkp_score === 5);
  } else {
    this.correctAnswerIndex = currentQuestionObj.options.findIndex((opt: any) => opt.correct);
    this.currentAnswerIsCorrect = currentQuestionObj.options[option].correct;
  }
  this.isAnswerChecked = true;
  this.answerExplanation = currentQuestionObj.explanation || currentQuestionObj.solution
    || 'Tidak ada penjelasan tersedia untuk soal ini.';
  this.showExplanation = true;
}
```

- [ ] **Step 3: Build**

```bash
cd /mnt/devarea/devarea/quiz-frontend
npm run build:prod 2>&1 | grep -E "ERROR|error TS|Output" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add src/app/question/question.component.ts
git commit -m "fix(tkp): TKP-aware calculateScore — sum 1-5 poin instead of binary correct/wrong

Score = rawPoints / maxPoints * 100 (backward compat percentage).
correct_answers = soal with 5 poin (best answer).
Study mode: shows poin earned for TKP, not binary green/red."
```

---

## Task 10: Frontend — Show TKP poin badge in review mode

**Files:**
- Modify: `quiz-frontend/src/app/question/question.component.html`

- [ ] **Step 1: Find where option correctness is visually indicated**

```bash
grep -n "correct\|isCorrect\|correctAnswer\|answer.*option\|option.*correct\|green\|wrong" \
  /mnt/devarea/devarea/quiz-frontend/src/app/question/question.component.html | head -20
```

- [ ] **Step 2: Add TKP poin badge to each option**

Find the option rendering block (where options are displayed with their letters A/B/C/D/E). Add a badge AFTER the option text for TKP:

```html
<!-- Existing option display pattern, add after option text span: -->
<span
  *ngIf="questionList[currentQuestion]?.question_type === 'tkp'
         && isAnswerChecked
         && questionList[currentQuestion]?.options[i]?.tkp_score"
  class="ml-2 inline-flex items-center justify-center w-6 h-6 rounded-full text-xs font-bold"
  [ngClass]="{
    'bg-green-500 text-white': questionList[currentQuestion]?.options[i]?.tkp_score === 5,
    'bg-yellow-400 text-white': questionList[currentQuestion]?.options[i]?.tkp_score === 4,
    'bg-orange-400 text-white': questionList[currentQuestion]?.options[i]?.tkp_score === 3,
    'bg-red-400 text-white':    questionList[currentQuestion]?.options[i]?.tkp_score <= 2
  }">
  {{ questionList[currentQuestion]?.options[i]?.tkp_score }}
</span>
```

Note: The exact template structure depends on existing HTML. Read the option loop in the template first and add the badge inside it.

- [ ] **Step 3: Build**

```bash
cd /mnt/devarea/devarea/quiz-frontend
npm run build:prod 2>&1 | grep -E "ERROR|error TS|Output" | head -10
```

- [ ] **Step 4: Commit + push**

```bash
cd /mnt/devarea/devarea/quiz-frontend
git add src/app/question/question.component.html
git commit -m "feat(tkp): show poin badge (1-5) per option in review/study mode for TKP soal"
git push prod feat/afd-157-simulasi-ujian
```

---

## Self-Review

### Spec Coverage

| Requirement | Task |
|---|---|
| Add `option_scores` JSON column | Task 1 |
| Parse 3 HTML formats and backfill | Task 2 |
| Update `question_type = 'tkp'` | Task 2 |
| Backend scoring uses 1-5 poin for TKP | Task 4 |
| Expose `option_scores` in paket soal response | Task 5 |
| Expose `option_scores` in simulasi response | Task 6 |
| Frontend maps `option_scores` from API | Task 8 |
| Frontend `calculateScore()` TKP-aware | Task 9 |
| Study mode no binary red/green for TKP | Task 9 |
| Badge poin in review mode | Task 10 |
| 45 soal without solution get default fallback | Task 2 (default_scores function) |

### Type Consistency

- `option_scores` field is `Option<serde_json::Value>` in Rust, `object | null` in TypeScript — consistent
- `opt_key` format `"opt1"..."opt5"` used in both backend JSON lookup and frontend `option_scores[optKey]` — consistent
- `tkp_score` on option objects in frontend, `option_scores` on question object — both used, consistent (score per option vs full map)

### Potential Issues

1. **`calculate_score` standard path** — user answers are `Vec<Option<i32>>` where index matches soal order. This is only correct if `paket_soal_items` order matches session order. Already the case for existing sessions — no change.

2. **`option_scores` JSON format in MySQL** — MySQL JSON stores as text; `row.try_get::<Option<serde_json::Value>, _>("option_scores")` works with SQLx's JSON feature. If SQLx feature flag `json` not enabled, may need `row.try_get::<Option<String>, _>("option_scores").ok().flatten().and_then(|s| serde_json::from_str(&s).ok())`.

   Check `Cargo.toml`:
   ```bash
   grep "sqlx" /mnt/devarea/devarea/rustproject/quiz-backend/Cargo.toml
   ```
   If `features = [..., "json"]` not present, use String parsing as fallback.

3. **Existing quiz sessions** — sessions completed before this fix will have wrong scores. Acceptable: old sessions are historical, new sessions will be correct.
