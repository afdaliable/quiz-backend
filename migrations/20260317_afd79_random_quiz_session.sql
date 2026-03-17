-- AFD-79: Random Quiz Session
-- Tambah session_type + question_ids ke quiz_sessions, jadikan paket_soal_id nullable

ALTER TABLE quiz_sessions
  MODIFY COLUMN paket_soal_id INT NULL,
  ADD COLUMN session_type VARCHAR(20) NOT NULL DEFAULT 'standard' AFTER nama_paket_soal,
  ADD COLUMN question_ids JSON NULL AFTER session_type;
