-- ───────────────────────────────────────────────
-- AFD-145: Rating soal (👍 berguna / 👎 membingungkan)
-- ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS question_ratings (
  question_id   INT          NOT NULL,
  user_id       CHAR(36)     NOT NULL,
  rating        ENUM('helpful', 'confusing') NOT NULL,
  created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (question_id, user_id),
  CONSTRAINT fk_qr_question FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE,
  CONSTRAINT fk_qr_user     FOREIGN KEY (user_id)     REFERENCES users(id) ON DELETE CASCADE
);

-- ───────────────────────────────────────────────
-- AFD-145: Laporan soal bermasalah
-- ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS question_reports (
  id            CHAR(36)     NOT NULL DEFAULT (UUID()),
  question_id   INT          NOT NULL,
  user_id       CHAR(36)     NOT NULL,
  reason        ENUM(
                  'wrong_answer',
                  'unclear_explanation',
                  'not_relevant',
                  'duplicate',
                  'other'
                ) NOT NULL,
  detail        TEXT         NULL,
  status        ENUM('pending', 'reviewed', 'resolved') NOT NULL DEFAULT 'pending',
  admin_note    TEXT         NULL,
  created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

  PRIMARY KEY (id),
  CONSTRAINT fk_qrep_question FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE,
  CONSTRAINT fk_qrep_user     FOREIGN KEY (user_id)     REFERENCES users(id) ON DELETE CASCADE,

  -- Batasi 1 laporan aktif (pending) per user per soal
  UNIQUE KEY uq_pending_report (question_id, user_id, status)
);

-- Index untuk admin dashboard
CREATE INDEX idx_qrep_question_status ON question_reports (question_id, status);
CREATE INDEX idx_qrep_status_created  ON question_reports (status, created_at DESC);
