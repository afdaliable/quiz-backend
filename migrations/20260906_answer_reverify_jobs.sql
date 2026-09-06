-- Pipeline for correcting answer keys that AI enrich itself flagged as
-- suspect (see soal_quality_controller's unverified-answers detector).
--
-- The safety rule this table records the outcome of: a proposed answer
-- coming from a note that says "I'm not sure" is NOT trustworthy on its
-- own -- that's the exact output which declared itself unreliable. So a
-- fresh, materi-library-grounded AI run is done independently, and the
-- stored key is only overwritten when that fresh run AGREES with the old
-- note's proposal. Two independent signals concurring. Anything else is
-- left for a human, counted in `needs_review`.
CREATE TABLE IF NOT EXISTS dbquizapp.answer_reverify_jobs (
    id              VARCHAR(36) PRIMARY KEY,
    status          ENUM('pending','running','completed','failed','cancelled')
                    NOT NULL DEFAULT 'pending',
    total           INT NOT NULL DEFAULT 0,
    processed       INT NOT NULL DEFAULT 0,
    -- fresh grounded run agreed with the note's proposal -> key updated
    applied         INT NOT NULL DEFAULT 0,
    -- disagreed, or no proposal to compare against -> left for a human
    needs_review    INT NOT NULL DEFAULT 0,
    -- fresh run reproduced the stored key -> nothing was wrong after all
    confirmed_ok    INT NOT NULL DEFAULT 0,
    failed_count    INT NOT NULL DEFAULT 0,
    -- per-soal outcome detail, JSON array
    results_json    LONGTEXT NULL,
    error_message   TEXT NULL,
    created_by      VARCHAR(255) NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at    TIMESTAMP NULL,

    INDEX idx_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
