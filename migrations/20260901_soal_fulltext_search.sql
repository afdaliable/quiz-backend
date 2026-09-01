-- Menu-Soal search (`GET /admin/soal/search?search=...`) ran `s.soal LIKE
-- '%term%'` -- a leading wildcard, so no index could be used, forcing a
-- full scan of the ~54k-row soal table for both the COUNT(*) and the
-- paginated SELECT on every keystroke (fixed separately on the frontend
-- to fire on submit, not per letter). Each request took 15s+ live.
--
-- FULLTEXT + BOOLEAN MODE with a trailing `*` supports the same
-- prefix-as-you-type search ("panca" matching "Pancasila") but via the
-- index instead of a table scan.
ALTER TABLE dbquizapp.soal
    ADD FULLTEXT INDEX ft_soal_text (soal),
    ALGORITHM=INPLACE, LOCK=SHARED;
