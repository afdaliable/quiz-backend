-- AFD-200: Taxonomy Hierarchy Tables
-- Creates 6 new tables: exam_tracks, categories, subcategories, topics, tags, question_tags

CREATE TABLE exam_tracks (
    id         CHAR(36)     NOT NULL DEFAULT (UUID()),
    slug       VARCHAR(100) NOT NULL,
    name       VARCHAR(255) NOT NULL,
    icon       VARCHAR(50)  NULL,
    status     ENUM('live', 'upcoming') NOT NULL DEFAULT 'upcoming',
    sort_order INT          NOT NULL DEFAULT 0,
    created_at TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id),
    UNIQUE INDEX idx_tracks_slug (slug),
    INDEX idx_tracks_status (status)
);

CREATE TABLE categories (
    id         CHAR(36)     NOT NULL DEFAULT (UUID()),
    track_id   CHAR(36)     NOT NULL,
    slug       VARCHAR(100) NOT NULL,
    name       VARCHAR(255) NOT NULL,
    sort_order INT          NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    INDEX idx_categories_track (track_id),
    UNIQUE INDEX idx_categories_track_slug (track_id, slug),
    CONSTRAINT fk_categories_track FOREIGN KEY (track_id) REFERENCES exam_tracks(id) ON DELETE CASCADE
);

CREATE TABLE subcategories (
    id          CHAR(36)     NOT NULL DEFAULT (UUID()),
    category_id CHAR(36)     NOT NULL,
    slug        VARCHAR(100) NOT NULL,
    name        VARCHAR(255) NOT NULL,
    sort_order  INT          NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    INDEX idx_subcategories_category (category_id),
    UNIQUE INDEX idx_subcategories_category_slug (category_id, slug),
    CONSTRAINT fk_subcategories_category FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE
);

CREATE TABLE topics (
    id               CHAR(36)     NOT NULL DEFAULT (UUID()),
    subcategory_id   CHAR(36)     NOT NULL,
    slug             VARCHAR(100) NOT NULL,
    name             VARCHAR(255) NOT NULL,
    sort_order       INT          NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    INDEX idx_topics_subcategory (subcategory_id),
    UNIQUE INDEX idx_topics_subcategory_slug (subcategory_id, slug),
    CONSTRAINT fk_topics_subcategory FOREIGN KEY (subcategory_id) REFERENCES subcategories(id) ON DELETE CASCADE
);

CREATE TABLE tags (
    id    CHAR(36)     NOT NULL DEFAULT (UUID()),
    slug  VARCHAR(100) NOT NULL,
    label VARCHAR(255) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE INDEX idx_tags_slug (slug)
);

CREATE TABLE question_tags (
    question_id INT      NOT NULL,
    tag_id      CHAR(36) NOT NULL,
    PRIMARY KEY (question_id, tag_id),
    INDEX idx_question_tags_tag (tag_id),
    CONSTRAINT fk_qt_question FOREIGN KEY (question_id) REFERENCES soal(id) ON DELETE CASCADE,
    CONSTRAINT fk_qt_tag     FOREIGN KEY (tag_id)      REFERENCES tags(id)  ON DELETE CASCADE
);
