-- Outbound email log for the notification system (one row per recipient send).
CREATE TABLE IF NOT EXISTS email_logs (
    id         BIGSERIAL PRIMARY KEY,
    to_email   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    status     TEXT NOT NULL,            -- 'sent' | 'failed' | 'dev'
    error      TEXT,
    sent_by    TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_email_logs_created ON email_logs (created_at DESC);
