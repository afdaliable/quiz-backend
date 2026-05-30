#!/usr/bin/env python3
"""
AFD-252: Backfill option_scores for TKP soal.
Run once: python3 scripts/backfill_tkp_scores.py
Reads solution HTML, parses known formats, writes JSON to option_scores.
"""
import re
import html
import json
import mysql.connector

DB = dict(
    host='100.87.162.99',
    user='root',
    password='love4JJI#123somuch',
    database='dbquizapp',
)

LETTER_TO_OPT = {'A': 'opt1', 'B': 'opt2', 'C': 'opt3', 'D': 'opt4', 'E': 'opt5'}
OPTS = ['opt1', 'opt2', 'opt3', 'opt4', 'opt5']


def strip_tags(text: str) -> str:
    """Remove HTML tags and decode entities (including &ndash; &nbsp; etc.)."""
    text = re.sub(r'<[^>]+>', ' ', text)
    text = html.unescape(text)
    return text


def _five_by_letter(matches) -> dict | None:
    """Given a list of (letter, score) tuples, return opt-keyed dict if 5 unique letters."""
    if len(matches) < 5:
        return None
    letters = [m[0].upper() for m in matches]
    if len(set(letters)) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in matches[:5]}
    # Duplicate letters — use positional order
    return {OPTS[i]: int(v) for i, (_, v) in enumerate(matches[:5])}


def parse_fmt1(solution: str):
    """Format: 'A: 2 poin'  (jadiasn, handles &nbsp; and <strong> wrappers)."""
    clean = strip_tags(solution)
    m = re.findall(r'([A-Ea-e])\s*:\s*([1-5])\s*(?:poin|point)', clean, re.IGNORECASE)
    if len(m) >= 5:
        return _five_by_letter(m)
    return None


def parse_fmt2(solution: str):
    """Format: 'Opsi A 2 poin' or 'Opsi a 2 poin'  (alfaiz, narrative).
    Also handles 'Opsi A 2' without poin keyword by trying a relaxed pattern.
    """
    clean = strip_tags(solution)
    # Strict: require poin/point keyword
    m = re.findall(r'Opsi\s+([A-Ea-e])\s+([1-5])\s+(?:poin|point)', clean, re.IGNORECASE)
    if len(m) >= 5:
        return _five_by_letter(m)
    # Relaxed: 'Opsi A 2' followed by space/non-digit (allows missing poin keyword)
    m2 = re.findall(r'Opsi\s+([A-Ea-e])\s+([1-5])(?!\d)', clean, re.IGNORECASE)
    if len(m2) >= 5:
        return _five_by_letter(m2)
    return None


def parse_fmt3(solution: str):
    """Format: 'A. teks – Nilai (2)' (handles &ndash;, case-insensitive Nilai)."""
    clean = strip_tags(solution)
    # html.unescape converts &ndash; to –, match both - and –
    m = re.findall(r'([A-E])\.\s+.*?[-–]\s*[Nn]ilai\s*\(([1-5])\)', clean, re.DOTALL | re.IGNORECASE)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def parse_fmt4(solution: str):
    """Format: 'Skor: A = 3' per line."""
    clean = strip_tags(solution)
    m = re.findall(r'\b([A-E])\s*=\s*([1-5])\b', clean, re.IGNORECASE)
    if len(m) >= 5:
        return _five_by_letter(m)
    return None


def parse_fmt5(solution: str):
    """Format: 'A. teks – nilai N' (no parentheses, handles en-dash)."""
    clean = strip_tags(solution)
    m = re.findall(r'([A-E])\.\s+.*?[-–]\s*[Nn]ilai\s+([1-5])(?!\d)', clean, re.DOTALL | re.IGNORECASE)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def parse_fmt6(solution: str):
    """Format: <li> items each containing '- Nilai (N)' — positional."""
    clean = strip_tags(solution)
    m = re.findall(r'[-–]\s*[Nn]ilai\s*\(([1-5])\)', clean, re.IGNORECASE)
    if len(m) == 5:
        return {OPTS[i]: int(v) for i, v in enumerate(m)}
    return None


def parse_fmt7(solution: str):
    """Format: 'A. text - (2)' (parens with no 'Nilai' keyword, handles en-dash)."""
    clean = strip_tags(solution)
    m = re.findall(r'([A-E])\.\s+.*?[-–]\s*\(([1-5])\)', clean, re.DOTALL | re.IGNORECASE)
    if len(m) == 5:
        return {LETTER_TO_OPT[l]: int(v) for l, v in m}
    return None


def parse_fmt8(solution: str):
    """Format: <li>text (N)</li> — score is just (N) at end of li item, positional."""
    # Extract content of each <li> then find last (N) in each
    items = re.findall(r'<li[^>]*>(.*?)</li>', solution, re.DOTALL | re.IGNORECASE)
    if len(items) >= 5:
        scores = []
        for item in items[:5]:
            text = strip_tags(item)
            nums = re.findall(r'\(([1-5])\)', text)
            if nums:
                scores.append(int(nums[-1]))
        if len(scores) == 5:
            return {OPTS[i]: v for i, v in enumerate(scores)}
    return None


def parse_fmt9(solution: str):
    """Format: 'A. N' (just letter dot bare score, no poin keyword).
    Matches lines like '<p>A. 1</p><p>B. 2</p>...'
    """
    clean = strip_tags(solution)
    # Match letter. whitespace single-digit score (not followed by another digit)
    m = re.findall(r'\b([A-E])\.\s+([1-5])(?!\d)', clean, re.IGNORECASE)
    if len(m) >= 5:
        return _five_by_letter(m)
    return None


def parse_fmt10(solution: str):
    """Format: narrative 'Opsi A (N poin)' or 'opsi A, N Poin' scattered in text."""
    clean = strip_tags(solution)
    # Match 'opsi X (N poin)' or 'opsi X, N Poin'
    m = re.findall(r'[Oo]psi\s+([A-E])[,\s]+\(?([1-5])\s*[Pp]oin\)?', clean, re.IGNORECASE)
    if len(m) >= 5:
        return _five_by_letter(m)
    return None


def parse_fmt11(solution: str):
    """Format: positional (N) extraction when exactly 5 parens with 1-5 scores found."""
    clean = strip_tags(solution)
    m = re.findall(r'\(([1-5])\)', clean)
    if len(m) == 5:
        return {OPTS[i]: int(v) for i, v in enumerate(m)}
    return None


def default_scores(correct_answer: str) -> dict:
    """Fallback for soal with no parseable solution. correct_answer=5, rest=4,3,2,1."""
    others = [o for o in OPTS if o != correct_answer]
    result = {correct_answer: 5}
    for i, opt in enumerate(others):
        result[opt] = 4 - i
    return result


def parse_scores(solution, correct_answer):
    if not solution or not solution.strip():
        return default_scores(correct_answer)
    parsed = (
        parse_fmt1(solution)
        or parse_fmt2(solution)
        or parse_fmt3(solution)
        or parse_fmt4(solution)
        or parse_fmt5(solution)
        or parse_fmt6(solution)
        or parse_fmt7(solution)
        or parse_fmt8(solution)
        or parse_fmt9(solution)
        or parse_fmt10(solution)
        or parse_fmt11(solution)
    )
    if parsed:
        return parsed
    # Fallback: solution exists but no parseable score format found.
    # Use default scoring based on correct_answer.
    return default_scores(correct_answer)


def alter_question_type_enum(cn):
    """Add 'tkp' to question_type ENUM if not already present."""
    cur = cn.cursor()
    cur.execute("SHOW COLUMNS FROM soal LIKE 'question_type'")
    row = cur.fetchone()
    cur.close()
    if row is None:
        return
    col_type = row[1].decode() if isinstance(row[1], bytes) else row[1]
    if 'tkp' not in col_type:
        print("Altering question_type ENUM to add 'tkp'...")
        alter_cur = cn.cursor()
        alter_cur.execute(
            "ALTER TABLE soal MODIFY COLUMN question_type "
            "ENUM('multiple_choice','true_false','fill_blank','tkp') NOT NULL DEFAULT 'multiple_choice'"
        )
        cn.commit()
        alter_cur.close()
        print("ENUM altered successfully.")
    else:
        print("question_type ENUM already contains 'tkp', skipping ALTER.")


def main():
    cn = mysql.connector.connect(**DB)

    # Step 0: ensure ENUM includes 'tkp'
    alter_question_type_enum(cn)

    cur = cn.cursor(dictionary=True)

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
        sol = row['solution'] or ''
        correct = row['correct_answer'] or 'opt1'
        scores = parse_scores(sol, correct)
        if scores and len(scores) == 5:
            update_cur.execute(
                "UPDATE soal SET option_scores = %s WHERE id = %s",
                (json.dumps(scores), row['id'])
            )
            # parsed = actual format found; skip = fell back to default_scores
            if not sol.strip():
                skip += 1
            else:
                # Try to detect if we actually parsed a format vs fell back
                test = (
                    parse_fmt1(sol) or parse_fmt2(sol) or parse_fmt3(sol) or
                    parse_fmt4(sol) or parse_fmt5(sol) or parse_fmt6(sol) or
                    parse_fmt7(sol) or parse_fmt8(sol) or parse_fmt9(sol) or
                    parse_fmt10(sol) or parse_fmt11(sol)
                )
                if test:
                    ok += 1
                else:
                    skip += 1  # used default_scores fallback
        else:
            print(f"  FAIL to parse id={row['id']}: {str(sol)[:120]}")
            fail += 1

    cn.commit()
    print(f"\nDone. format_parsed={ok}  default_fallback={skip}  failed={fail}")
    print(f"Total updated: {ok + skip} / {len(rows)}")

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

    # Final verification
    cur.execute("""
        SELECT
            COUNT(*) AS total,
            SUM(option_scores IS NOT NULL) AS has_scores,
            SUM(question_type = 'tkp') AS is_tkp_type,
            SUM(option_scores IS NULL) AS missing
        FROM soal
        WHERE subcategory_id IN (SELECT id FROM subcategories WHERE slug='tkp-tes-karakteristik-pribadi')
        AND status='active'
    """)
    stats = cur.fetchone()
    print(f"\nVerification: total={stats['total']} has_scores={stats['has_scores']} "
          f"is_tkp_type={stats['is_tkp_type']} missing={stats['missing']}")

    cur.close()
    update_cur.close()
    cn.close()


if __name__ == '__main__':
    main()
