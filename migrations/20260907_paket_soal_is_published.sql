-- Explicit publish flag for paket_soal.
--
-- The admin UI has had a "toggle published" button wired for a while, but
-- POST /admin/packages/{id}/toggle-published returned 404 -- the endpoint,
-- the column semantics, and the public-side filter were all missing.
--
-- Until now, whether a package appears in the quiz app was decided by
-- accident: the public listing queries INNER JOIN kategori_soal, so a
-- package with a NULL kategori_id silently vanishes. That conflates "not
-- categorised yet" with "not meant to be visible" -- confirmed live, one
-- package (id 22, "Tes Wawasan Kebangsaan - UPKP") is currently hidden
-- from app.nagih.id purely because its category was never filled in.
--
-- paket_soal.status int(11) already existed but is NULL on all 30 rows and
-- referenced nowhere in the codebase; reusing an int with no defined
-- meaning would just invite the next person to guess. A named boolean is
-- honest about what it controls.
--
-- DEFAULT 1 and the explicit backfill are the safety-critical part: 29
-- packages are live in the app right now, and a default of 0 would make
-- every one of them disappear the moment the filter ships.
ALTER TABLE dbquizapp.paket_soal
    ADD COLUMN is_published TINYINT(1) NOT NULL DEFAULT 1 AFTER is_premium;

UPDATE dbquizapp.paket_soal SET is_published = 1;
