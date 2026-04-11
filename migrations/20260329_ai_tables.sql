-- ─── AFD-153: AI Feature Tables ─────────────────────────────────────────────
-- Run this migration before deploying AFD-151 / AFD-152 features.

-- ─── 1. ai_usage_logs ────────────────────────────────────────────────────────
-- Every AI provider call is logged here (single enrich + bulk enrich)
CREATE TABLE IF NOT EXISTS dbquizapp.ai_usage_logs (
    id                BIGINT AUTO_INCREMENT PRIMARY KEY,
    question_id       BIGINT NOT NULL,
    job_id            VARCHAR(36) NULL,           -- NULL for single enrich
    provider          VARCHAR(20) NOT NULL,        -- 'deepseek' | 'gemini'
    model             VARCHAR(50) NOT NULL,        -- 'deepseek-chat' | 'gemini-2.0-flash'
    fields_requested  JSON NOT NULL,               -- ["solution", "tag"]
    prompt_tokens     INT NOT NULL DEFAULT 0,
    completion_tokens INT NOT NULL DEFAULT 0,
    cost_estimate_usd DECIMAL(10,6) NOT NULL DEFAULT 0,
    success           BOOLEAN NOT NULL DEFAULT TRUE,
    error_message     TEXT NULL,
    created_by_admin  VARCHAR(255) NULL,           -- admin email from Authentik
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_question_id (question_id),
    INDEX idx_job_id (job_id),
    INDEX idx_created_at (created_at),
    INDEX idx_provider (provider)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;


-- ─── 2. ai_generated_content ─────────────────────────────────────────────────
-- AI-generated field values, before and after admin review
CREATE TABLE IF NOT EXISTS dbquizapp.ai_generated_content (
    id                BIGINT AUTO_INCREMENT PRIMARY KEY,
    question_id       BIGINT NOT NULL,
    job_id            VARCHAR(36) NULL,
    field_name        VARCHAR(50) NOT NULL,        -- 'solution' | 'tag' | 'modul' | 'pelajaran'
    original_value    TEXT NULL,                   -- value before enrichment
    generated_value   TEXT NOT NULL,               -- value produced by AI
    provider          VARCHAR(20) NOT NULL,
    model             VARCHAR(50) NOT NULL,
    accepted          BOOLEAN NOT NULL DEFAULT FALSE,
    accepted_at       TIMESTAMP NULL,
    accepted_by       VARCHAR(255) NULL,           -- admin email
    rejected          BOOLEAN NOT NULL DEFAULT FALSE,
    rejected_at       TIMESTAMP NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_question_id (question_id),
    INDEX idx_job_id (job_id),
    INDEX idx_field_name (field_name),
    INDEX idx_accepted (accepted),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;


-- ─── 3. ai_bulk_jobs ─────────────────────────────────────────────────────────
-- Progress tracking for batch AI enrichment jobs
CREATE TABLE IF NOT EXISTS dbquizapp.ai_bulk_jobs (
    id                VARCHAR(36) PRIMARY KEY,     -- UUID
    status            ENUM('pending','running','completed','failed','cancelled')
                      NOT NULL DEFAULT 'pending',
    total             INT NOT NULL DEFAULT 0,
    processed         INT NOT NULL DEFAULT 0,
    succeeded         INT NOT NULL DEFAULT 0,
    failed            INT NOT NULL DEFAULT 0,
    fields            JSON NOT NULL,               -- ["solution", "tag"]
    auto_save         BOOLEAN NOT NULL DEFAULT FALSE,
    rate_limit_per_minute INT NOT NULL DEFAULT 20,
    filter_json       JSON NULL,
    error_message     TEXT NULL,
    created_by        VARCHAR(255) NULL,
    created_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    started_at        TIMESTAMP NULL,
    completed_at      TIMESTAMP NULL,

    INDEX idx_status (status),
    INDEX idx_created_by (created_by),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
