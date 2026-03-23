-- AFD-141: Add Pomodoro session stats columns to quiz_sessions
ALTER TABLE quiz_sessions
  ADD COLUMN pomodoro_enabled TINYINT(1) NOT NULL DEFAULT 0,
  ADD COLUMN pomodoro_sessions INT NOT NULL DEFAULT 0,
  ADD COLUMN pomodoro_focus_minutes INT NOT NULL DEFAULT 0,
  ADD COLUMN pomodoro_questions_answered INT NOT NULL DEFAULT 0;

-- AFD-141: Add preferences JSON column to users
ALTER TABLE users
  ADD COLUMN preferences JSON NULL;
