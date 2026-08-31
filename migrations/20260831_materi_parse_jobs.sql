-- "Import Soal dari Teks" -- admin pastes raw extracted text (OCR/copy-paste
-- from an existing latihan-soal document, questions+options already
-- present, just unstructured), backend splits it into per-question blocks
-- deterministically (see materi_parse_controller::split_into_blocks), AI
-- only fills correct_answer + solution per block, never invents new
-- questions. Same async-job reasoning as materi_generate_jobs.
CREATE TABLE IF NOT EXISTS dbquizapp.materi_parse_jobs (
    id             VARCHAR(36) PRIMARY KEY,
    status         ENUM('pending','running','completed','failed') NOT NULL DEFAULT 'pending',
    total_blocks   INT NOT NULL DEFAULT 0,
    skipped_json   LONGTEXT NULL,   -- blocks that couldn't be split/parsed at all (e.g. image-only)
    accepted_json  LONGTEXT NULL,   -- ParsedSoal[] that passed validation
    rejected_json  LONGTEXT NULL,   -- AI output that failed validation (empty field, bad correct_answer, etc.)
    error_message  TEXT NULL,
    created_by     VARCHAR(255) NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at   TIMESTAMP NULL,

    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
