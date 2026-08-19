CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT,
    display_name TEXT,
    avatar_url TEXT,
    avatar_local_path TEXT
);

CREATE TABLE tweets (
    id TEXT PRIMARY KEY,
    author_id TEXT,
    text TEXT NOT NULL DEFAULT '',
    created_at TEXT,
    canonical_url TEXT NOT NULL,
    raw_json TEXT,
    enrichment_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (enrichment_status IN ('pending', 'ok', 'deleted', 'protected', 'suspended', 'unavailable', 'rate_limited', 'failed')),
    enrichment_error TEXT,
    enriched_at TEXT,
    imported_at TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (author_id) REFERENCES users(id)
);

CREATE TABLE likes (
    tweet_id TEXT PRIMARY KEY,
    liked_at TEXT,
    archive_text TEXT,
    archive_source TEXT,
    FOREIGN KEY (tweet_id) REFERENCES tweets(id) ON DELETE CASCADE
);

CREATE TABLE quotes (
    tweet_id TEXT PRIMARY KEY,
    quoted_id TEXT,
    text TEXT NOT NULL DEFAULT '',
    canonical_url TEXT,
    author_name TEXT,
    username TEXT,
    FOREIGN KEY (tweet_id) REFERENCES tweets(id) ON DELETE CASCADE
);

CREATE TABLE media (
    id TEXT PRIMARY KEY,
    tweet_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('image', 'video', 'gif', 'unknown')),
    remote_url TEXT,
    preview_url TEXT,
    local_path TEXT,
    thumbnail_path TEXT,
    width INTEGER,
    height INTEGER,
    duration_ms INTEGER,
    FOREIGN KEY (tweet_id) REFERENCES tweets(id) ON DELETE CASCADE
);

CREATE INDEX idx_tweets_author ON tweets(author_id);
CREATE INDEX idx_tweets_created ON tweets(created_at DESC);
CREATE INDEX idx_tweets_status ON tweets(enrichment_status);
CREATE INDEX idx_media_tweet_kind ON media(tweet_id, kind);
CREATE INDEX idx_likes_liked ON likes(liked_at DESC);
CREATE INDEX idx_users_username ON users(username COLLATE NOCASE);

CREATE VIRTUAL TABLE tweet_fts USING fts5(
    tweet_id UNINDEXED,
    text,
    archive_text,
    username,
    display_name,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER tweets_fts_insert AFTER INSERT ON tweets BEGIN
    INSERT INTO tweet_fts(tweet_id, text, archive_text, username, display_name)
    VALUES (
        NEW.id,
        NEW.text,
        COALESCE((SELECT archive_text FROM likes WHERE tweet_id = NEW.id), ''),
        COALESCE((SELECT username FROM users WHERE id = NEW.author_id), ''),
        COALESCE((SELECT display_name FROM users WHERE id = NEW.author_id), '')
    );
END;

CREATE TRIGGER tweets_fts_update AFTER UPDATE OF text, author_id ON tweets BEGIN
    DELETE FROM tweet_fts WHERE tweet_id = OLD.id;
    INSERT INTO tweet_fts(tweet_id, text, archive_text, username, display_name)
    VALUES (
        NEW.id,
        NEW.text,
        COALESCE((SELECT archive_text FROM likes WHERE tweet_id = NEW.id), ''),
        COALESCE((SELECT username FROM users WHERE id = NEW.author_id), ''),
        COALESCE((SELECT display_name FROM users WHERE id = NEW.author_id), '')
    );
END;

CREATE TRIGGER tweets_fts_delete AFTER DELETE ON tweets BEGIN
    DELETE FROM tweet_fts WHERE tweet_id = OLD.id;
END;

CREATE TRIGGER likes_fts_insert AFTER INSERT ON likes BEGIN
    UPDATE tweet_fts SET archive_text = COALESCE(NEW.archive_text, '') WHERE tweet_id = NEW.tweet_id;
END;

CREATE TRIGGER likes_fts_update AFTER UPDATE OF archive_text ON likes BEGIN
    UPDATE tweet_fts SET archive_text = COALESCE(NEW.archive_text, '') WHERE tweet_id = NEW.tweet_id;
END;

CREATE TRIGGER users_fts_insert AFTER INSERT ON users BEGIN
    UPDATE tweet_fts
    SET username = COALESCE(NEW.username, ''), display_name = COALESCE(NEW.display_name, '')
    WHERE tweet_id IN (SELECT id FROM tweets WHERE author_id = NEW.id);
END;

CREATE TRIGGER users_fts_update AFTER UPDATE OF username, display_name ON users BEGIN
    UPDATE tweet_fts
    SET username = COALESCE(NEW.username, ''), display_name = COALESCE(NEW.display_name, '')
    WHERE tweet_id IN (SELECT id FROM tweets WHERE author_id = NEW.id);
END;
