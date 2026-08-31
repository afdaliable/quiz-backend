-- Phase 2 of the soal analytics work: retroactive AI-classify sweep over
-- the existing soal corpus (confirmed live: 20259 of ~54k soal have no
-- track/subcategory at all). Each item is 3 sequential AI calls
-- (classify_subcategory, classify_topic, classify_additional_topics) via
-- classify_and_save, so a strictly-sequential rate-limited loop like
-- ai_bulk_jobs uses would take on the order of 1-2 weeks for 20k rows.
-- This job runs a bounded number of items concurrently (see `concurrency`)
-- instead, tracked in its own table since it doesn't need ai_bulk_jobs'
-- fields/auto_save columns (classify always does all 3 stages and always
-- saves).
CREATE TABLE IF NOT EXISTS dbquizapp.taxonomy_classify_bulk_jobs (
    id             VARCHAR(36) PRIMARY KEY,
    status         ENUM('pending','running','completed','failed','cancelled')
                   NOT NULL DEFAULT 'pending',
    mode           VARCHAR(32) NOT NULL DEFAULT 'uncategorized',
    total          INT NOT NULL DEFAULT 0,
    processed      INT NOT NULL DEFAULT 0,
    succeeded      INT NOT NULL DEFAULT 0,
    failed         INT NOT NULL DEFAULT 0,
    concurrency    INT NOT NULL DEFAULT 10,
    error_message  TEXT NULL,
    created_by     VARCHAR(255) NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at     TIMESTAMP NULL,
    completed_at   TIMESTAMP NULL,

    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
