-- Create quiz_sessions table for persistent quiz session management
CREATE TABLE IF NOT EXISTS quiz_sessions (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    paket_soal_id INT NOT NULL,
    kategori_soal VARCHAR(255) NOT NULL,
    nama_paket_soal VARCHAR(255) NOT NULL,
    current_question INT DEFAULT 0,
    answers TEXT,
    marked_questions TEXT,
    time_remaining INT,
    total_time INT,
    is_completed BOOLEAN DEFAULT FALSE,
    score INT DEFAULT 0,
    correct_answers INT DEFAULT 0,
    incorrect_answers INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_quiz_sessions_user_id (user_id),
    INDEX idx_quiz_sessions_paket_soal_id (paket_soal_id),
    INDEX idx_quiz_sessions_is_completed (is_completed),
    INDEX idx_quiz_sessions_created_at (created_at),
    
    CONSTRAINT fk_quiz_sessions_user_id 
        FOREIGN KEY (user_id) REFERENCES users(id) 
        ON DELETE CASCADE
);