-- BookOrbit 兼容同步：标注/笔记、书签、阅读状态、页面阅读统计（均按用户）

CREATE TABLE book_annotations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    text TEXT NOT NULL DEFAULT '',
    note TEXT,
    chapter TEXT,
    drawer TEXT NOT NULL DEFAULT 'lighten',
    color TEXT,
    pos_format TEXT NOT NULL CHECK (pos_format IN ('xpointer', 'pdf')),
    pos0 TEXT NOT NULL,
    pos1 TEXT,
    pageno INTEGER,
    -- KOReader 身份时间（无时区 `YYYY-MM-DD HH:MM:SS`），与 pos0 一起构成设备端 key
    device_datetime TEXT NOT NULL,
    device_updated TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    -- 仅在内容变化时更新
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX idx_book_annotations_user_book ON book_annotations(user_id, book_id);
CREATE INDEX idx_book_annotations_identity ON book_annotations(user_id, book_id, device_datetime, pos0);

CREATE TABLE annotation_sync_states (
    annotation_id INTEGER NOT NULL REFERENCES book_annotations(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    external_key TEXT NOT NULL,
    external_datetime TEXT,
    -- 0 = 已下发但设备未确认
    last_applied_version INTEGER NOT NULL DEFAULT 0,
    delete_acked_at TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (annotation_id, device_id)
);

CREATE INDEX idx_annotation_sync_states_device ON annotation_sync_states(device_id, external_key);

CREATE TABLE book_bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    pos TEXT NOT NULL,
    pageno INTEGER,
    chapter TEXT,
    note TEXT,
    title TEXT NOT NULL,
    device_datetime TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX idx_book_bookmarks_user_book ON book_bookmarks(user_id, book_id);

CREATE TABLE bookmark_sync_links (
    bookmark_id INTEGER NOT NULL REFERENCES book_bookmarks(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    external_key TEXT NOT NULL,
    device_datetime TEXT,
    applied_at TEXT NOT NULL,
    PRIMARY KEY (bookmark_id, device_id)
);

CREATE INDEX idx_bookmark_sync_links_device ON bookmark_sync_links(device_id, external_key);

CREATE TABLE user_book_status (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('reading', 'complete', 'abandoned')),
    -- `YYYY-MM-DD`
    status_modified TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, book_id)
);

CREATE TABLE reading_page_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    page INTEGER NOT NULL,
    start_time INTEGER NOT NULL,
    duration_seconds INTEGER NOT NULL,
    total_pages INTEGER NOT NULL,
    UNIQUE (user_id, book_id, page, start_time)
);

CREATE INDEX idx_reading_page_stats_user_book ON reading_page_stats(user_id, book_id, start_time);
