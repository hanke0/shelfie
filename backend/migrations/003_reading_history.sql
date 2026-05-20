CREATE TABLE reading_progress_history (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    percent REAL,
    current_page INTEGER,
    last_position TEXT,
    source TEXT NOT NULL CHECK (source IN ('web', 'koreader')),
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_reading_history_user_time
    ON reading_progress_history(user_id, recorded_at DESC);

CREATE INDEX idx_reading_history_user_library_time
    ON reading_progress_history(user_id, library_id, recorded_at DESC);
