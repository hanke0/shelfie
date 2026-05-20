CREATE TABLE user_reading_progress (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    percent REAL,
    current_page INTEGER,
    last_position TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, book_id)
);

CREATE INDEX idx_user_reading_progress_user_time
    ON user_reading_progress(user_id, updated_at DESC);

CREATE INDEX idx_user_reading_progress_user_library_time
    ON user_reading_progress(user_id, library_id, updated_at DESC);
