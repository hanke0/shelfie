ALTER TABLE users ADD COLUMN password_md5 TEXT;

CREATE TABLE koreader_progress (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    document TEXT NOT NULL,
    progress TEXT NOT NULL,
    percentage REAL NOT NULL,
    device TEXT,
    device_id TEXT,
    timestamp INTEGER NOT NULL,
    book_id TEXT REFERENCES books(id) ON DELETE SET NULL,
    metadata_json TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, document)
);

CREATE TABLE koreader_document_links (
    document TEXT NOT NULL PRIMARY KEY,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    link_source TEXT NOT NULL DEFAULT 'manual' CHECK (link_source IN ('manual', 'md5')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_koreader_progress_book ON koreader_progress(book_id);
CREATE INDEX idx_koreader_progress_user_updated ON koreader_progress(user_id, updated_at DESC);
