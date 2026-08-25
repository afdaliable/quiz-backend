-- Progress tracking for the "generate soal from materi text" feature.
-- Same shape as ai_bulk_jobs (async job -> poll for result), needed because
-- generate_soal() waits on the same slow 9router calls that made single
-- enrich need async job handling -- a synchronous multipart POST here would
-- hit the same Cloudflare Free-plan proxy timeout.
CREATE TABLE IF NOT EXISTS dbquizapp.materi_generate_jobs (
    id                VARCHAR(36) PRIMARY KEY,     -- UUID
    status            ENUM('pending','running','completed','failed')
                      NOT NULL DEFAULT 'pending',
    materi            VARCHAR(255) NOT NULL,
    topic_group       VARCHAR(255) NOT NULL,
    count_requested   INT NOT NULL,
    accepted_json     JSON NULL,                   -- Vec<GeneratedSoal> once completed
    rejected_json     JSON NULL,                   -- Vec<{reason, raw}> once completed
    error_message     TEXT NULL,
    created_by        VARCHAR(255) NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at      TIMESTAMP NULL,

    INDEX idx_status (status),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
