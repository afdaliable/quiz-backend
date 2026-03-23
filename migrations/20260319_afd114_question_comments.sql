-- AFD-114: Comment System API
-- Creates question_comments and comment_upvotes tables

CREATE TABLE IF NOT EXISTS question_comments (
    id           CHAR(36)  NOT NULL PRIMARY KEY,
    question_id  INT       NOT NULL,
    user_id      CHAR(36)  NOT NULL,
    parent_id    CHAR(36)  NULL,
    body         TEXT      NOT NULL,
    is_admin_pin BOOLEAN   NOT NULL DEFAULT FALSE,
    upvotes      INT       NOT NULL DEFAULT 0,
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id)     REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id)   REFERENCES question_comments(id) ON DELETE CASCADE,
    INDEX idx_qc_question_created (question_id, created_at),
    INDEX idx_qc_parent_id        (parent_id)
);

CREATE TABLE IF NOT EXISTS comment_upvotes (
    comment_id CHAR(36) NOT NULL,
    user_id    CHAR(36) NOT NULL,
    PRIMARY KEY (comment_id, user_id),
    FOREIGN KEY (comment_id) REFERENCES question_comments(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id)    REFERENCES users(id) ON DELETE CASCADE
);
