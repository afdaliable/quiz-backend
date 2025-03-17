-- Add is_premium column to paket_soal table
ALTER TABLE dbquizapp.paket_soal ADD COLUMN is_premium BOOLEAN NOT NULL DEFAULT FALSE;

-- Update existing records (optional)
-- You can set specific packages as premium here if needed
-- Example: UPDATE dbquizapp.paket_soal SET is_premium = TRUE WHERE id IN (1, 2, 3); 