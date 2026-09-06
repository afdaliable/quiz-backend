-- exam_tracks.icon is meant to hold an emoji -- the admin hierarchy page
-- has a free-text icon input and the TS type documents it as "emoji" --
-- but the column was utf8mb3, which cannot encode anything outside the
-- BMP. Confirmed live: creating a track with "🧪" returns
--   1366 (22007): Incorrect string value: '\xF0\x9F\xA7\xAA' for column 'icon'
-- i.e. a 500 to the admin. Existing rows show only 'book' or NULL, so no
-- emoji has ever successfully been saved through this field.
--
-- Only the icon column is widened; the rest of the table stays utf8mb3 so
-- the char(36) id keeps its collation and existing foreign keys from
-- categories.track_id are untouched.
ALTER TABLE dbquizapp.exam_tracks
    MODIFY COLUMN icon VARCHAR(50) CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci NULL;
