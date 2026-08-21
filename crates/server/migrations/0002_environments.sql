-- 环境配置和 Agent 心跳元数据；不保存 Docker 运行时资源或固定 online 状态
CREATE TABLE IF NOT EXISTS environments (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL CHECK (length(trim(name)) > 0),
    kind          TEXT NOT NULL CHECK (kind IN ('local', 'agent')),
    endpoint      TEXT NOT NULL,
    agent_id      TEXT UNIQUE,
    last_seen_at  TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_environments_kind ON environments (kind);
CREATE INDEX IF NOT EXISTS idx_environments_last_seen_at ON environments (last_seen_at);
