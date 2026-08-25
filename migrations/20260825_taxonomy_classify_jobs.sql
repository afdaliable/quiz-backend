-- Progress tracking for AI-driven taxonomy classification (subcategory,
-- topic, and topik-tambahan cross-link). Same async-job reasoning as
-- materi_generate_jobs -- 2-3 sequential LLM calls need to run outside a
-- synchronous request that would otherwise trip Cloudflare's proxy timeout.
CREATE TABLE IF NOT EXISTS dbquizapp.taxonomy_classify_jobs (
    id                VARCHAR(36) PRIMARY KEY,     -- UUID
    status            ENUM('pending','running','completed','failed')
                      NOT NULL DEFAULT 'pending',
    question_id       BIGINT NOT NULL,
    result_json       JSON NULL,                   -- ClassifyResult once completed
    error_message     TEXT NULL,
    created_by        VARCHAR(255) NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at      TIMESTAMP NULL,

    INDEX idx_status (status),
    INDEX idx_question_id (question_id),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
