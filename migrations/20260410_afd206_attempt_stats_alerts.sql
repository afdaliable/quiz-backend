-- AFD-206: Materialized attempt stats + admin alerts
-- Implements Opsi B: tabel materialisasi per soal, diisi saat quiz complete

-- Tabel statistik attempt per soal
CREATE TABLE IF NOT EXISTS question_attempt_stats (
  question_id    INT     NOT NULL,
  total_attempts INT     NOT NULL DEFAULT 0,
  correct_count  INT     NOT NULL DEFAULT 0,
  total_time_sec BIGINT  NOT NULL DEFAULT 0,
  last_updated   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (question_id),
  CONSTRAINT fk_qas_soal FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE
);

-- Tabel notifikasi admin (misalnya: mismatch difficulty)
CREATE TABLE IF NOT EXISTS admin_alerts (
  id          CHAR(36)     NOT NULL DEFAULT (UUID()),
  type        VARCHAR(50)  NOT NULL,
  entity_type VARCHAR(50)  NOT NULL,
  entity_id   VARCHAR(50)  NOT NULL,
  detail      JSON         NOT NULL,
  is_read     BOOLEAN      NOT NULL DEFAULT FALSE,
  created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  INDEX idx_alerts_type_read (type, is_read),
  INDEX idx_alerts_entity (entity_type, entity_id)
);
