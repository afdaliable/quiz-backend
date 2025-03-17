-- Add is_premium flag to paket_soal table
ALTER TABLE dbquizapp.paket_soal ADD COLUMN is_premium BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'Flag to indicate if this quiz package is premium-only';

-- Update existing quiz packages to be free by default
UPDATE dbquizapp.paket_soal SET is_premium = FALSE;

-- Create index for faster querying
CREATE INDEX idx_paket_soal_is_premium ON dbquizapp.paket_soal(is_premium); 