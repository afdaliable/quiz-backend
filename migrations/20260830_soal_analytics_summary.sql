-- Precomputed soal coverage/quality summary -- Phase 1 of the soal
-- analytics dashboard. Computing these aggregates live per page view is
-- fine cost-wise (a GROUP BY over ~54k rows is fast), but this table lets
-- the admin page load instantly and gives a stable snapshot to compare
-- day-to-day as more soal get added/categorized. Recomputed daily by a
-- background task (see main.rs) and on-demand via POST
-- /admin/soal-analytics/recompute.

CREATE TABLE IF NOT EXISTS dbquizapp.soal_analytics_coverage (
    id             BIGINT AUTO_INCREMENT PRIMARY KEY,
    level          ENUM('track','category','subcategory','topic') NOT NULL,
    node_id        VARCHAR(36) NOT NULL,
    node_name      VARCHAR(255) NOT NULL,
    parent_path    VARCHAR(500) NULL,       -- e.g. "SKD > TIU" for display
    track_id       VARCHAR(36) NULL,
    category_id    VARCHAR(36) NULL,
    subcategory_id VARCHAR(36) NULL,
    total_count    INT NOT NULL DEFAULT 0,
    easy_count     INT NOT NULL DEFAULT 0,
    medium_count   INT NOT NULL DEFAULT 0,
    hard_count     INT NOT NULL DEFAULT 0,
    active_count   INT NOT NULL DEFAULT 0,
    draft_count    INT NOT NULL DEFAULT 0,
    archived_count INT NOT NULL DEFAULT 0,
    computed_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_level (level),
    INDEX idx_node (node_id),
    INDEX idx_subcategory (subcategory_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Tag-string inconsistency detector: `soal.tag` is free comma-separated
-- text, not a taxonomy FK, so the same intended tag drifts across rows
-- ("TIU (Tes Intelegensi Umum)" vs "TIU(Tes Intelegensi umum)" vs "Tiu").
-- Each row here is one normalized-token -> one raw-variant-seen mapping;
-- a normalized_tag with more than one row is a cleanup candidate.
CREATE TABLE IF NOT EXISTS dbquizapp.soal_analytics_tag_variants (
    id             BIGINT AUTO_INCREMENT PRIMARY KEY,
    normalized_tag VARCHAR(500) NOT NULL,
    variant_text   VARCHAR(500) NOT NULL,
    soal_count     INT NOT NULL DEFAULT 0,
    computed_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_normalized (normalized_tag(191))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- Single-row top-level snapshot (total soal, uncategorized count, last run).
CREATE TABLE IF NOT EXISTS dbquizapp.soal_analytics_meta (
    id                    INT PRIMARY KEY DEFAULT 1,
    last_computed_at      TIMESTAMP NULL,
    total_soal            INT NOT NULL DEFAULT 0,
    total_uncategorized   INT NOT NULL DEFAULT 0,
    total_distinct_tags   INT NOT NULL DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
