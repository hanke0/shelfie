DROP TABLE IF EXISTS refresh_jobs;

CREATE TABLE maintenance_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    tool TEXT NOT NULL,
    library_id TEXT REFERENCES libraries(id) ON DELETE CASCADE,
    lock_key TEXT NOT NULL,
    params TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'cancelled', 'interrupted')),
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    progress_current INTEGER NOT NULL DEFAULT 0,
    progress_total INTEGER,
    progress_message TEXT,
    result TEXT,
    error TEXT,
    created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT
);

-- 同一 lock_key 同时只允许一个运行中的任务
CREATE UNIQUE INDEX idx_maintenance_tasks_running_lock
    ON maintenance_tasks(lock_key) WHERE status = 'running';
CREATE INDEX idx_maintenance_tasks_started ON maintenance_tasks(started_at DESC);
