-- AFD-253: Add navigation_mode and sections_json to exam_simulations
-- navigation_mode: 'free' (SKD/BUMN/STAN) | 'section_locked' (PPPK/LPDP)
-- sections_json: JSON array [{name, count, section_duration_minutes?}] from simulasi_templates.sections
ALTER TABLE exam_simulations
  ADD COLUMN navigation_mode VARCHAR(20) NOT NULL DEFAULT 'free',
  ADD COLUMN sections_json JSON NULL;
