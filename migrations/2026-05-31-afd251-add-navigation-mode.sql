-- AFD-251: Add navigation_mode to simulasi_templates
-- free = bebas loncat antar section kapan saja (SKD, RBB, STAN)
-- section_locked = setelah selesai 1 section tidak bisa balik (PPPK, LPDP)
ALTER TABLE simulasi_templates
  ADD COLUMN navigation_mode ENUM('free', 'section_locked') NOT NULL DEFAULT 'free';
