-- AFD-224: Passage-Based Questions — tabel passages & FK ke soal
-- Safe for production: passage_id NULL = soal biasa, semua data existing tetap valid

CREATE TABLE passages (
  id         INT AUTO_INCREMENT PRIMARY KEY,
  content    TEXT         NOT NULL,
  title      VARCHAR(255) NULL,
  source     VARCHAR(255) NULL,
  language   VARCHAR(20)  NULL DEFAULT 'id',
  created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

ALTER TABLE soal
  ADD COLUMN passage_id INT NULL AFTER id,
  ADD INDEX idx_soal_passage (passage_id),
  ADD CONSTRAINT fk_soal_passage
      FOREIGN KEY (passage_id) REFERENCES passages(id) ON DELETE SET NULL;
