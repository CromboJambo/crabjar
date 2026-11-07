-- Crabjar Email Integration Schema
-- Local SQLite working model; provider is source of truth.
-- Supports multi-account, FTS5 search, approval workflow, recurrence tracking.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- Accounts: one row per email account (work/personal separation)
CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,          -- gmail, outlook, imap, smtp
    email_address TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    config_json TEXT NOT NULL,       -- encrypted credentials & server settings
    is_active BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Messages: core message storage (one row per provider UID)
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    uid TEXT NOT NULL,               -- provider-specific unique ID
    thread_id TEXT,                  -- conversation threading
    in_reply_to TEXT,                -- raw In-Reply-To header
    references TEXT,                 -- raw References header
    from_address TEXT NOT NULL,
    to_addresses TEXT NOT NULL,      -- JSON array of {address, name}
    cc_addresses TEXT DEFAULT '[]',  -- JSON array
    bcc_addresses TEXT DEFAULT '[]', -- JSON array
    subject TEXT NOT NULL DEFAULT '',
    body_text TEXT,                  -- plain text body
    body_html TEXT,                  -- HTML body
    date_sent DATETIME NOT NULL,     -- original send time
    date_received DATETIME DEFAULT CURRENT_TIMESTAMP,  -- local receipt time
    flags INTEGER DEFAULT 0,         -- bitfield: seen=1, answered=2, flagged=4, deleted=8
    size_bytes INTEGER,
    headers_json TEXT,               -- raw headers for reference
    is_draft BOOLEAN DEFAULT 0,
    synced_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, uid)
);

-- Labels/Folders: Gmail-style labels or IMAP folder hierarchy
CREATE TABLE IF NOT EXISTS labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    name TEXT NOT NULL,
    type TEXT DEFAULT 'custom',      -- folder, label, system, custom
    parent_id INTEGER REFERENCES labels(id),  -- for folder hierarchy
    color TEXT,
    UNIQUE(account_id, name)
);

-- Message-Label mapping (many-to-many)
CREATE TABLE IF NOT EXISTS message_labels (
    message_id INTEGER NOT NULL REFERENCES messages(id),
    label_id INTEGER NOT NULL REFERENCES labels(id),
    PRIMARY KEY (message_id, label_id)
);

-- Contacts: extracted from message headers
CREATE TABLE IF NOT EXISTS contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_address TEXT NOT NULL UNIQUE,
    display_name TEXT,
    organization TEXT,
    notes TEXT,
    last_seen_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Attachments: stored separately with local file references
CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL REFERENCES messages(id),
    filename TEXT NOT NULL,
    content_type TEXT,
    size_bytes INTEGER,
    stored_path TEXT,                -- local filesystem path
    is_inline BOOLEAN DEFAULT 0,
    cid TEXT                         -- Content-ID for inline references
);

-- Agent Decisions: log of agent actions/analysis on messages
CREATE TABLE IF NOT EXISTS agent_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    message_id INTEGER REFERENCES messages(id),  -- nullable for account-wide decisions
    action_type TEXT NOT NULL,       -- classify, summarize, draft, suggest_send, etc.
    input_json TEXT,                 -- what the agent was given
    output_json TEXT NOT NULL,       -- agent's decision/output
    confidence_score REAL,           -- 0.0 to 1.0
    status TEXT DEFAULT 'completed', -- completed, pending_approval, approved, rejected, executed
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    resolved_at DATETIME
);

-- Approval Requests: explicit approval workflow state
CREATE TABLE IF NOT EXISTS approval_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    decision_id INTEGER REFERENCES agent_decisions(id),
    action_type TEXT NOT NULL,       -- send, delete, move, archive, reply
    target_message_id INTEGER REFERENCES messages(id),
    details_json TEXT NOT NULL,      -- what would happen if approved
    status TEXT DEFAULT 'pending',   -- pending, approved, rejected, expired
    requested_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    resolved_at DATETIME,
    resolved_by TEXT                 -- user or automation identifier
);

-- Recurrence Rules: for recurring events/tasks extracted from email
CREATE TABLE IF NOT EXISTS recurrence_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_message_id INTEGER REFERENCES messages(id),
    rule_type TEXT NOT NULL,         -- daily, weekly, monthly, custom
    frequency_json TEXT NOT NULL,    -- detailed recurrence specification
    next_occurrence_at DATETIME,
    last_occurrence_at DATETIME,
    is_active BOOLEAN DEFAULT 1
);

-- FTS5 Full-Text Search Index
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    subject,
    body_text,
    from_address,
    to_addresses,
    content='messages',
    content_rowid='id'
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, subject, body_text, from_address, to_addresses)
    VALUES (new.id, new.subject, new.body_text, new.from_address, new.to_addresses);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
    UPDATE messages_fts SET subject=new.subject, body_text=new.body_text,
        from_address=new.from_address, to_addresses=new.to_addresses
    WHERE rowid = new.id;
END;

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_messages_account ON messages(account_id);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_date_sent ON messages(date_sent);
CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_address);
CREATE INDEX IF NOT EXISTS idx_labels_account ON labels(account_id);
CREATE INDEX IF NOT EXISTS idx_approvals_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_decisions_status ON agent_decisions(status);