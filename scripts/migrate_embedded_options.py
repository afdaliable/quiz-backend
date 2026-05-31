#!/usr/bin/env python3
"""
AFD-255: Migrate soal that have answer options embedded in the `soal` text
(opt1-opt5 empty) into proper opt1-opt5 columns.

Handles two textual formats:
  - "stem A. opt B. opt C. opt D. opt E. opt"
  - "stem (A) opt (B) opt (C) opt (D) opt (E) opt"

Safety guards (skipped, left for manual review):
  - soal contains an <img> tag (real image-based question)
  - parsed options are image placeholders ("Gambar N" / "Pilihan N")
  - fewer than 2 parsed options

Usage:
  python3 migrate_embedded_options.py            # dry-run (no writes), prints stats + samples
  python3 migrate_embedded_options.py --apply    # perform UPDATEs
  python3 migrate_embedded_options.py --limit 50 # restrict scope (useful for testing)
"""
import re
import sys
import html
import argparse
import mysql.connector

DB = dict(
    host="100.87.162.99",
    user="root",
    password="love4JJI#123somuch",
    database="dbquizapp",
)

# Two option formats. Each captures: stem, A, B, C, (D optional), (E optional)
PATTERNS = [
    re.compile(
        r'^(.*?)\s+A\.\s+(.+?)\s+B\.\s+(.+?)\s+C\.\s+(.+?)'
        r'(?:\s+D\.\s+(.+?))?(?:\s+E\.\s+(.+?))?$'
    ),
    re.compile(
        r'^(.*?)\s+\(A\)\s+(.+?)\s+\(B\)\s+(.+?)\s+\(C\)\s+(.+?)'
        r'(?:\s+\(D\)\s+(.+?))?(?:\s+\(E\)\s+(.+?))?$'
    ),
]

TAG_RE = re.compile(r'<[^>]+>')
WS_RE = re.compile(r'\s+')
IMG_PLACEHOLDER_RE = re.compile(r'^(gambar|pilihan|opsi)\s*\d*$', re.IGNORECASE)


def strip_html(text: str) -> str:
    text = html.unescape(text or "")
    text = TAG_RE.sub(' ', text)
    text = WS_RE.sub(' ', text)
    return text.strip()


def is_junk_option(o: str) -> bool:
    # ellipsis-only or too short to be a real answer
    stripped = o.strip(' .…')
    return len(stripped) < 2


def parse_options(clean: str):
    """Return (stem, [opts]) if parseable, else None."""
    for pat in PATTERNS:
        m = pat.match(clean)
        if not m:
            continue
        stem = (m.group(1) or "").strip()
        opts = [g.strip() for g in m.groups()[1:] if g and g.strip()]
        if len(opts) >= 2 and stem and not any(is_junk_option(o) for o in opts):
            return stem, opts
    return None


def is_image_soal(raw: str, opts) -> bool:
    if '<img' in (raw or '').lower():
        return True
    if opts and all(IMG_PLACEHOLDER_RE.match(o) for o in opts):
        return True
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--apply', action='store_true', help='perform UPDATEs (default: dry-run)')
    ap.add_argument('--limit', type=int, default=0, help='limit number of soal scanned')
    args = ap.parse_args()

    conn = mysql.connector.connect(**DB)
    cur = conn.cursor()

    sql = (
        "SELECT id, soal FROM soal "
        "WHERE (opt1 IS NULL OR opt1 = '') AND status = 'active'"
    )
    if args.limit > 0:
        sql += f" LIMIT {args.limit}"
    cur.execute(sql)
    rows = cur.fetchall()

    total = len(rows)
    parseable = []   # (id, stem, opts)
    image_soal = 0
    unparseable = 0

    for soal_id, soal in rows:
        clean = strip_html(soal)
        parsed = parse_options(clean)
        if parsed is None:
            unparseable += 1
            continue
        stem, opts = parsed
        if is_image_soal(soal, opts):
            image_soal += 1
            continue
        parseable.append((soal_id, stem, opts))

    print(f"Total soal with empty opt1: {total}")
    print(f"  Parseable (will update): {len(parseable)}")
    print(f"  Image soal (skipped):    {image_soal}")
    print(f"  Unparseable (skipped):   {unparseable}")
    print()

    # Show a few samples of what would change
    print("=== SAMPLE (first 3 parseable) ===")
    for soal_id, stem, opts in parseable[:3]:
        print(f"ID {soal_id}")
        print(f"  stem: {stem[:90]}")
        for i, o in enumerate(opts):
            print(f"  opt{i+1}: {o[:70]}")
        print()

    if not args.apply:
        print("DRY-RUN — no changes written. Re-run with --apply to execute.")
        cur.close()
        conn.close()
        return

    print(f"Applying {len(parseable)} updates...")
    updated = 0
    for soal_id, stem, opts in parseable:
        padded = (opts + [None, None, None, None, None])[:5]
        cur.execute(
            "UPDATE soal SET soal=%s, opt1=%s, opt2=%s, opt3=%s, opt4=%s, opt5=%s "
            "WHERE id=%s",
            (stem, padded[0], padded[1], padded[2], padded[3], padded[4], soal_id),
        )
        updated += 1
        if updated % 200 == 0:
            conn.commit()
            print(f"  committed {updated}...")
    conn.commit()
    print(f"Done. Updated {updated} soal.")

    cur.close()
    conn.close()


if __name__ == '__main__':
    main()
