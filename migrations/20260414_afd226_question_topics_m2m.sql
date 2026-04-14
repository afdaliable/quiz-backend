-- AFD-226: Multi-topic classification for questions (M2M)
-- Adds question_topics pivot table so one question can belong to multiple topics
-- Pattern mirrors existing question_tags table

CREATE TABLE IF NOT EXISTS question_topics (
    question_id INT      NOT NULL,
    topic_id    CHAR(36) NOT NULL,
    PRIMARY KEY (question_id, topic_id),
    INDEX idx_qt_topic (topic_id),
    CONSTRAINT fk_qtopic_question FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE,
    CONSTRAINT fk_qtopic_topic    FOREIGN KEY (topic_id)    REFERENCES topics(id) ON DELETE CASCADE
);

-- Migrate existing direct FK assignments into the pivot table
INSERT IGNORE INTO question_topics (question_id, topic_id)
SELECT id, topic_id FROM soal WHERE topic_id IS NOT NULL;
