-- AFD-202: Seed data for taxonomy hierarchy (9 exam tracks)
-- Idempotent: safe to re-run. Uses COALESCE to reuse existing UUIDs.
-- Run AFTER migrations/20260408_afd200_taxonomy_hierarchy.sql

-- ─── Exam Tracks ────────────────────────────────────────────────────────────

SET @track_masuk_asn  = COALESCE((SELECT id FROM exam_tracks WHERE slug='masuk-asn'),  UUID());
SET @track_ptn        = COALESCE((SELECT id FROM exam_tracks WHERE slug='ptn-mandiri'), UUID());
SET @track_profesi    = COALESCE((SELECT id FROM exam_tracks WHERE slug='profesi'),     UUID());
SET @track_bahasa     = COALESCE((SELECT id FROM exam_tracks WHERE slug='bahasa'),      UUID());
SET @track_beasiswa   = COALESCE((SELECT id FROM exam_tracks WHERE slug='beasiswa'),    UUID());
SET @track_bumn       = COALESCE((SELECT id FROM exam_tracks WHERE slug='bumn'),        UUID());
SET @track_karir_asn  = COALESCE((SELECT id FROM exam_tracks WHERE slug='karir-asn'),  UUID());
SET @track_kedinasan  = COALESCE((SELECT id FROM exam_tracks WHERE slug='kedinasan'),   UUID());
SET @track_snbt       = COALESCE((SELECT id FROM exam_tracks WHERE slug='snbt-utbk'),  UUID());

INSERT IGNORE INTO exam_tracks (id, slug, name, status, sort_order) VALUES
    (@track_masuk_asn, 'masuk-asn',   'Seleksi Masuk ASN',       'live',     1),
    (@track_ptn,       'ptn-mandiri', 'PTN Mandiri',              'upcoming', 2),
    (@track_profesi,   'profesi',     'Profesi & Sertifikasi',    'upcoming', 3),
    (@track_bahasa,    'bahasa',      'Bahasa & Internasional',   'upcoming', 4),
    (@track_beasiswa,  'beasiswa',    'Beasiswa',                 'upcoming', 5),
    (@track_bumn,      'bumn',        'Seleksi Non-PNS (BUMN)',   'upcoming', 6),
    (@track_karir_asn, 'karir-asn',   'Karir ASN',                'live',     7),
    (@track_kedinasan, 'kedinasan',   'Kedinasan',                'live',     8),
    (@track_snbt,      'snbt-utbk',   'SNBT & UTBK',              'live',     9);

-- ─── Categories: masuk-asn ──────────────────────────────────────────────────

SET @cat_skd   = COALESCE((SELECT id FROM categories WHERE track_id=@track_masuk_asn AND slug='skd'),  UUID());
SET @cat_skb   = COALESCE((SELECT id FROM categories WHERE track_id=@track_masuk_asn AND slug='skb'),  UUID());
SET @cat_pppk  = COALESCE((SELECT id FROM categories WHERE track_id=@track_masuk_asn AND slug='pppk'), UUID());
SET @cat_upkp  = COALESCE((SELECT id FROM categories WHERE track_id=@track_masuk_asn AND slug='upkp'), UUID());

INSERT IGNORE INTO categories (id, track_id, slug, name, sort_order) VALUES
    (@cat_skd,  @track_masuk_asn, 'skd',  'SKD (Seleksi Kompetensi Dasar)',    1),
    (@cat_skb,  @track_masuk_asn, 'skb',  'SKB (Seleksi Kompetensi Bidang)',   2),
    (@cat_pppk, @track_masuk_asn, 'pppk', 'PPPK',                             3),
    (@cat_upkp, @track_masuk_asn, 'upkp', 'UPKP',                             4);

-- ─── Subcategories: SKD ────────────────────────────────────────────────────

SET @sub_twk = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_skd AND slug='twk'), UUID());
SET @sub_tiu = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_skd AND slug='tiu'), UUID());
SET @sub_tkp = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_skd AND slug='tkp'), UUID());

INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_twk, @cat_skd, 'twk', 'TWK (Tes Wawasan Kebangsaan)',     1),
    (@sub_tiu, @cat_skd, 'tiu', 'TIU (Tes Intelegensia Umum)',       2),
    (@sub_tkp, @cat_skd, 'tkp', 'TKP (Tes Karakteristik Pribadi)',   3);

-- ─── Subcategories: SKB ─────────────────────────────────────────────────────

SET @sub_skb_jabatan = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_skb AND slug='kompetensi-jabatan'), UUID());
INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_skb_jabatan, @cat_skb, 'kompetensi-jabatan', 'Kompetensi Jabatan', 1);

-- ─── Subcategories: PPPK ───────────────────────────────────────────────────

SET @sub_pppk_guru      = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_pppk AND slug='guru'),      UUID());
SET @sub_pppk_teknis    = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_pppk AND slug='teknis'),    UUID());
SET @sub_pppk_kesehatan = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_pppk AND slug='kesehatan'), UUID());

INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_pppk_guru,      @cat_pppk, 'guru',      'PPPK Guru',      1),
    (@sub_pppk_teknis,    @cat_pppk, 'teknis',    'PPPK Teknis',    2),
    (@sub_pppk_kesehatan, @cat_pppk, 'kesehatan', 'PPPK Kesehatan', 3);

-- ─── Subcategories: UPKP ────────────────────────────────────────────────────

SET @sub_upkp = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_upkp AND slug='kompetensi-upkp'), UUID());
INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_upkp, @cat_upkp, 'kompetensi-upkp', 'Kompetensi UPKP', 1);

-- ─── Topics: TWK ────────────────────────────────────────────────────────────

SET @top_pancasila    = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='pancasila'),       UUID());
SET @top_uud1945      = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='uud-1945'),        UUID());
SET @top_nkri         = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='nkri'),            UUID());
SET @top_sejarah      = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='sejarah'),         UUID());
SET @top_bahasa_indo  = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='bahasa-indonesia'),UUID());
SET @top_pilar_negara = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_twk AND slug='pilar-negara'),    UUID());

INSERT IGNORE INTO topics (id, subcategory_id, slug, name, sort_order) VALUES
    (@top_pancasila,    @sub_twk, 'pancasila',        'Pancasila',        1),
    (@top_uud1945,      @sub_twk, 'uud-1945',         'UUD 1945',         2),
    (@top_nkri,         @sub_twk, 'nkri',             'NKRI',             3),
    (@top_sejarah,      @sub_twk, 'sejarah',           'Sejarah Nasional', 4),
    (@top_bahasa_indo,  @sub_twk, 'bahasa-indonesia', 'Bahasa Indonesia', 5),
    (@top_pilar_negara, @sub_twk, 'pilar-negara',     'Pilar Negara',     6);

-- ─── Topics: TIU ────────────────────────────────────────────────────────────

SET @top_verbal   = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tiu AND slug='verbal'),  UUID());
SET @top_numerik  = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tiu AND slug='numerik'), UUID());
SET @top_figural  = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tiu AND slug='figural'), UUID());

INSERT IGNORE INTO topics (id, subcategory_id, slug, name, sort_order) VALUES
    (@top_verbal,  @sub_tiu, 'verbal',  'Kemampuan Verbal',  1),
    (@top_numerik, @sub_tiu, 'numerik', 'Kemampuan Numerik', 2),
    (@top_figural, @sub_tiu, 'figural', 'Kemampuan Figural', 3);

-- ─── Topics: TKP (7 aspek) ──────────────────────────────────────────────────

SET @top_pelayanan_publik = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='pelayanan-publik'), UUID());
SET @top_jejaring_kerja   = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='jejaring-kerja'),   UUID());
SET @top_sosial_budaya    = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='sosial-budaya'),    UUID());
SET @top_profesionalisme  = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='profesionalisme'),  UUID());
SET @top_anti_radikalisme = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='anti-radikalisme'), UUID());
SET @top_tik              = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='tik'),              UUID());
SET @top_bela_negara      = COALESCE((SELECT id FROM topics WHERE subcategory_id=@sub_tkp AND slug='bela-negara'),      UUID());

INSERT IGNORE INTO topics (id, subcategory_id, slug, name, sort_order) VALUES
    (@top_pelayanan_publik, @sub_tkp, 'pelayanan-publik', 'Pelayanan Publik',    1),
    (@top_jejaring_kerja,   @sub_tkp, 'jejaring-kerja',   'Jejaring Kerja',      2),
    (@top_sosial_budaya,    @sub_tkp, 'sosial-budaya',    'Sosial Budaya',       3),
    (@top_profesionalisme,  @sub_tkp, 'profesionalisme',  'Profesionalisme',     4),
    (@top_anti_radikalisme, @sub_tkp, 'anti-radikalisme', 'Anti-Radikalisme',    5),
    (@top_tik,              @sub_tkp, 'tik',              'Teknologi Informasi', 6),
    (@top_bela_negara,      @sub_tkp, 'bela-negara',      'Bela Negara',         7);

-- ─── Categories: PTN Mandiri ────────────────────────────────────────────────

SET @cat_snbt = COALESCE((SELECT id FROM categories WHERE track_id=@track_ptn AND slug='snbt'), UUID());
INSERT IGNORE INTO categories (id, track_id, slug, name, sort_order) VALUES
    (@cat_snbt, @track_ptn, 'snbt', 'SNBT', 1);

SET @sub_pu    = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt AND slug='penalaran-umum'),   UUID());
SET @sub_mat   = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt AND slug='matematika'),        UUID());
SET @sub_bind  = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt AND slug='bahasa-indonesia'), UUID());
SET @sub_bing  = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt AND slug='bahasa-inggris'),   UUID());

INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_pu,   @cat_snbt, 'penalaran-umum',   'Penalaran Umum',   1),
    (@sub_mat,  @cat_snbt, 'matematika',        'Matematika',       2),
    (@sub_bind, @cat_snbt, 'bahasa-indonesia',  'Bahasa Indonesia', 3),
    (@sub_bing, @cat_snbt, 'bahasa-inggris',    'Bahasa Inggris',   4);

-- ─── Categories: SNBT & UTBK ────────────────────────────────────────────────

SET @cat_snbt_utbk = COALESCE((SELECT id FROM categories WHERE track_id=@track_snbt AND slug='utbk'), UUID());
INSERT IGNORE INTO categories (id, track_id, slug, name, sort_order) VALUES
    (@cat_snbt_utbk, @track_snbt, 'utbk', 'UTBK', 1);

SET @sub_snbt_pu   = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt_utbk AND slug='penalaran-umum'),       UUID());
SET @sub_snbt_mat  = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt_utbk AND slug='matematika'),            UUID());
SET @sub_snbt_bind = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt_utbk AND slug='bahasa-indonesia'),      UUID());
SET @sub_snbt_bing = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt_utbk AND slug='bahasa-inggris'),        UUID());
SET @sub_snbt_pmk  = COALESCE((SELECT id FROM subcategories WHERE category_id=@cat_snbt_utbk AND slug='pengetahuan-mapel'),     UUID());

INSERT IGNORE INTO subcategories (id, category_id, slug, name, sort_order) VALUES
    (@sub_snbt_pu,   @cat_snbt_utbk, 'penalaran-umum',   'Penalaran Umum',             1),
    (@sub_snbt_mat,  @cat_snbt_utbk, 'matematika',        'Matematika',                 2),
    (@sub_snbt_bind, @cat_snbt_utbk, 'bahasa-indonesia',  'Bahasa Indonesia',           3),
    (@sub_snbt_bing, @cat_snbt_utbk, 'bahasa-inggris',    'Bahasa Inggris',             4),
    (@sub_snbt_pmk,  @cat_snbt_utbk, 'pengetahuan-mapel', 'Pengetahuan Mata Pelajaran', 5);

-- ─── Categories: Profesi ────────────────────────────────────────────────────

SET @cat_ppg    = COALESCE((SELECT id FROM categories WHERE track_id=@track_profesi AND slug='ppg-utn'), UUID());
SET @cat_ukmppd = COALESCE((SELECT id FROM categories WHERE track_id=@track_profesi AND slug='ukmppd'),  UUID());

INSERT IGNORE INTO categories (id, track_id, slug, name, sort_order) VALUES
    (@cat_ppg,    @track_profesi, 'ppg-utn', 'PPG / UTN', 1),
    (@cat_ukmppd, @track_profesi, 'ukmppd',  'UKMPPD',    2);

-- ─── Categories: Bahasa ─────────────────────────────────────────────────────

SET @cat_toefl = COALESCE((SELECT id FROM categories WHERE track_id=@track_bahasa AND slug='toefl-itp'), UUID());
SET @cat_ielts = COALESCE((SELECT id FROM categories WHERE track_id=@track_bahasa AND slug='ielts'),      UUID());

INSERT IGNORE INTO categories (id, track_id, slug, name, sort_order) VALUES
    (@cat_toefl, @track_bahasa, 'toefl-itp', 'TOEFL ITP', 1),
    (@cat_ielts, @track_bahasa, 'ielts',     'IELTS',     2);

-- ─── Tags (INSERT IGNORE on unique slug) ────────────────────────────────────

-- TWK / Pancasila
INSERT IGNORE INTO tags (id, slug, label) VALUES
    (UUID(), 'sila-1',             'Sila ke-1 (Ketuhanan)'),
    (UUID(), 'sila-2',             'Sila ke-2 (Kemanusiaan)'),
    (UUID(), 'sila-3',             'Sila ke-3 (Persatuan)'),
    (UUID(), 'sila-4',             'Sila ke-4 (Kerakyatan)'),
    (UUID(), 'sila-5',             'Sila ke-5 (Keadilan)'),
    (UUID(), 'butir-pancasila',    'Butir-Butir Pancasila'),
    (UUID(), 'historis-pancasila', 'Historis Pancasila');

-- TIU / Verbal
INSERT IGNORE INTO tags (id, slug, label) VALUES
    (UUID(), 'analogi-kata',     'Analogi Kata'),
    (UUID(), 'sinonim',          'Sinonim'),
    (UUID(), 'antonim',          'Antonim'),
    (UUID(), 'kelompok-kata',    'Kelompok Kata'),
    (UUID(), 'penalaran-verbal', 'Penalaran Verbal');

-- TIU / Numerik
INSERT IGNORE INTO tags (id, slug, label) VALUES
    (UUID(), 'deret-angka',  'Deret Angka'),
    (UUID(), 'aritmetika',   'Aritmetika'),
    (UUID(), 'perbandingan', 'Perbandingan'),
    (UUID(), 'aljabar',      'Aljabar');

-- TKP
INSERT IGNORE INTO tags (id, slug, label) VALUES
    (UUID(), 'pelayanan-publik',  'Pelayanan Publik'),
    (UUID(), 'integritas',        'Integritas'),
    (UUID(), 'kerjasama',         'Kerjasama Tim'),
    (UUID(), 'profesionalisme',   'Profesionalisme'),
    (UUID(), 'anti-radikalisme',  'Anti-Radikalisme');
