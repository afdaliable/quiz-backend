-- AFD-133: Daily Challenge — Database tables
-- Migration: 20260323_afd133_daily_challenge

CREATE TABLE IF NOT EXISTS daily_challenges (
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    challenge_date DATE NOT NULL UNIQUE,
    soal_id INT NOT NULL,
    created_by VARCHAR(36) NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (soal_id) REFERENCES soal(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS daily_challenge_attempts (
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    challenge_date DATE NOT NULL,
    soal_id INT NOT NULL,
    selected_answer TINYINT NOT NULL,
    is_correct TINYINT(1) NOT NULL DEFAULT 0,
    time_taken_ms INT NOT NULL DEFAULT 0,
    score INT NOT NULL DEFAULT 0,
    answered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_user_date (user_id, challenge_date),
    INDEX idx_challenge_date (challenge_date),
    INDEX idx_user_id (user_id),
    FOREIGN KEY (soal_id) REFERENCES soal(id) ON DELETE CASCADE
);
