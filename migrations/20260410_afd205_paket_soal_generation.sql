-- AFD-205: Extend paket_soal with generation metadata
-- Adds is_generated, generation_rules, difficulty_mix columns
-- All additive — no existing columns modified or dropped

ALTER TABLE paket_soal
  ADD COLUMN is_generated    BOOLEAN NOT NULL DEFAULT FALSE
      COMMENT 'TRUE jika paket dibuat via generate endpoint',
  ADD COLUMN generation_rules JSON NULL
      COMMENT 'Rules yang digunakan saat generate: track_id, category_id, dst',
  ADD COLUMN difficulty_mix   JSON NULL
      COMMENT 'Distribusi soal per difficulty: {"easy":N,"medium":N,"hard":N}';
