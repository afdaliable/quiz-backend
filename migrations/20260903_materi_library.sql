-- RAG grounding for AI enrich: without a reference source, the AI answers
-- exam questions from its own general knowledge, which is generic and can
-- miss source-specific wording tricks (confirmed live: a soal about "Etos
-- Kerja" needed the exact Modul Etika PNS text -- Franz Magnis-Suseno 2002
-- -- to catch a deliberate word-swap distractor; the AI without that source
-- got it wrong and hedged with a self-flagged "perlu verifikasi" note).
--
-- materi_library holds admin-curated reference text scoped to a
-- subcategory (required) and optionally a narrower topic. AI enrich looks
-- up matching materi for a soal's subcategory_id/topic_id and injects it
-- into the prompt as grounding context.
-- subcategory_id/topic_id use utf8mb3 to match subcategories.id/topics.id
-- exactly (FK columns must share charset+collation); title/content/source
-- are forced to utf8mb4 individually so admin-entered reference text can
-- contain emoji/full unicode without that constraint.
CREATE TABLE IF NOT EXISTS dbquizapp.materi_library (
    id             VARCHAR(36) CHARACTER SET utf8mb4 PRIMARY KEY DEFAULT (UUID()),
    subcategory_id CHAR(36) NOT NULL,
    topic_id       CHAR(36) NULL,
    title          VARCHAR(255) CHARACTER SET utf8mb4 NOT NULL,
    content        LONGTEXT CHARACTER SET utf8mb4 NOT NULL,
    source         VARCHAR(500) CHARACTER SET utf8mb4 NULL,
    created_by     VARCHAR(255) CHARACTER SET utf8mb4 NULL,
    created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_materi_library_subcategory (subcategory_id),
    INDEX idx_materi_library_topic (topic_id),
    CONSTRAINT fk_materi_library_subcategory FOREIGN KEY (subcategory_id) REFERENCES dbquizapp.subcategories(id) ON DELETE CASCADE,
    CONSTRAINT fk_materi_library_topic FOREIGN KEY (topic_id) REFERENCES dbquizapp.topics(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb3;
