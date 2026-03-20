-- AFD-120: Add onboarding fields to users table
ALTER TABLE users
  ADD COLUMN onboarding_completed BOOLEAN     NOT NULL DEFAULT FALSE,
  ADD COLUMN onboarding_goals     TEXT        NULL,
  ADD COLUMN exam_timeframe       VARCHAR(50) NULL,
  ADD COLUMN target_exam_date     DATE        NULL;

-- Existing users skip onboarding wizard — they already know the product
UPDATE users SET onboarding_completed = TRUE WHERE created_at < NOW();
