-- AFD-46: Add question_type discriminator column to soal table
-- All existing questions default to 'multiple_choice' — no data update needed.

ALTER TABLE soal
  ADD COLUMN question_type ENUM('multiple_choice', 'true_false', 'fill_blank')
  NOT NULL DEFAULT 'multiple_choice'
  AFTER soal;
