-- Progress tracking for the merged-options bulk fix (soal_quality_controller).
-- No AI involved (pure deterministic regex split), but looping a few hundred
-- sequential row UPDATEs inside one synchronous request still trips
-- Cloudflare's proxy timeout on the Free plan -- confirmed live (504 on
-- POST /admin/soal-quality/merged-options/fix with ~660 rows). Same
-- async-job pattern as materi_generate_jobs / taxonomy_classify_jobs.
CREATE TABLE IF NOT EXISTS dbquizapp.soal_quality_fix_jobs (
    id                VARCHAR(36) PRIMARY KEY,     -- UUID
    status            ENUM('pending','running','completed','failed')
                      NOT NULL DEFAULT 'pending',
    fixed             INT NULL,
    skipped_not_auto_fixable INT NULL,
    failed_count      INT NULL,
    error_message     TEXT NULL,
    created_by        VARCHAR(255) NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at      TIMESTAMP NULL,

    INDEX idx_status (status),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
