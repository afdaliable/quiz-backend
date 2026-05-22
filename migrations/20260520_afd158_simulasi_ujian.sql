-- AFD-158: Simulasi Ujian
-- Tabel exam_simulations, simulasi_user_attempts + ALTER quiz_sessions tambah simulasi_id

CREATE TABLE IF NOT EXISTS exam_simulations (
    id INT AUTO_INCREMENT PRIMARY KEY,
    nama_simulasi VARCHAR(255) NOT NULL,
    deskripsi TEXT NULL,
    paket_soal_id INT NULL
        COMMENT 'Diisi jika generation_mode=paket',
    generation_mode VARCHAR(40) NOT NULL DEFAULT 'paket'
        COMMENT 'paket | subcategory_composition | topic_composition | random_pool',
    generation_config JSON NULL
        COMMENT 'Konfigurasi komposisi (compositions[], difficulty_mix, dst)',
    duration_minutes INT NOT NULL,
    total_questions INT NOT NULL,
    passing_score INT NOT NULL DEFAULT 60
        COMMENT 'Persen 0-100',
    is_premium BOOLEAN NOT NULL DEFAULT FALSE,
    max_attempts INT NOT NULL DEFAULT 0
        COMMENT '0 = unlimited',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_exam_sim_active (is_active),
    INDEX idx_exam_sim_paket (paket_soal_id),

    CONSTRAINT fk_exam_sim_paket
        FOREIGN KEY (paket_soal_id) REFERENCES paket_soal(id)
        ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS simulasi_user_attempts (
    id INT AUTO_INCREMENT PRIMARY KEY,
    simulasi_id INT NOT NULL,
    -- Match users.id (CHAR(36) utf8mb3) and quiz_sessions.user_id (VARCHAR utf8mb3)
    user_id CHAR(36) CHARACTER SET utf8mb3 COLLATE utf8mb3_general_ci NOT NULL,
    quiz_session_id VARCHAR(36) NOT NULL,
    attempt_number INT NOT NULL,
    score INT NULL,
    is_passed BOOLEAN NULL,
    completed_at TIMESTAMP NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_sim_att_user (user_id),
    INDEX idx_sim_att_sim (simulasi_id),
    INDEX idx_sim_att_session (quiz_session_id),
    UNIQUE KEY uq_sim_att_session (quiz_session_id),

    CONSTRAINT fk_sim_att_sim
        FOREIGN KEY (simulasi_id) REFERENCES exam_simulations(id)
        ON DELETE CASCADE,
    CONSTRAINT fk_sim_att_user
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE CASCADE
);

ALTER TABLE quiz_sessions
  ADD COLUMN simulasi_id INT NULL AFTER session_type,
  ADD INDEX idx_quiz_sessions_simulasi (simulasi_id);
