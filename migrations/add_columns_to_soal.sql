-- Add new columns to soal table
ALTER TABLE dbquizapp.soal 
ADD COLUMN sumberfile VARCHAR(500) DEFAULT NULL,
ADD COLUMN modul VARCHAR(500) DEFAULT NULL,
ADD COLUMN pelajaran VARCHAR(500) DEFAULT NULL; 