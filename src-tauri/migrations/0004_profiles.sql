DROP TRIGGER IF EXISTS tweets_fts_insert;
DROP TRIGGER IF EXISTS tweets_fts_update;
DROP TRIGGER IF EXISTS tweets_fts_delete;
DROP TRIGGER IF EXISTS likes_fts_insert;
DROP TRIGGER IF EXISTS likes_fts_update;
DROP TRIGGER IF EXISTS likes_fts_delete;
DROP TRIGGER IF EXISTS users_fts_insert;
DROP TRIGGER IF EXISTS users_fts_update;

CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    username TEXT,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    avatar_local_path TEXT
);

CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO profiles(id, display_name)
VALUES ('__legacy__', 'Imported archive');

ALTER TABLE likes RENAME TO likes_legacy;

CREATE TABLE likes (
    profile_id TEXT NOT NULL,
    tweet_id TEXT NOT NULL,
    liked_at TEXT,
    archive_text TEXT,
    archive_source TEXT,
    PRIMARY KEY (profile_id, tweet_id),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    FOREIGN KEY (tweet_id) REFERENCES tweets(id) ON DELETE CASCADE
);

INSERT INTO likes(profile_id, tweet_id, liked_at, archive_text, archive_source)
SELECT '__legacy__', tweet_id, liked_at, archive_text, archive_source
FROM likes_legacy;
INSERT INTO app_settings(key, value)
SELECT 'active_profile_id', '__legacy__'
WHERE EXISTS(SELECT 1 FROM likes WHERE profile_id = '__legacy__');
DROP TABLE likes_legacy;

CREATE INDEX idx_likes_profile_liked ON likes(profile_id, liked_at DESC);
CREATE INDEX idx_likes_source ON likes(profile_id, archive_source);
CREATE INDEX idx_profiles_username ON profiles(username COLLATE NOCASE);

DELETE FROM tweet_fts;
INSERT INTO tweet_fts(rowid, tweet_id, text, archive_text, username, display_name)
SELECT
    tweets.rowid,
    tweets.id,
    tweets.text,
    '',
    COALESCE(users.username, ''),
    COALESCE(users.display_name, '')
FROM tweets
LEFT JOIN users ON users.id = tweets.author_id;

CREATE VIRTUAL TABLE like_fts USING fts5(
    profile_id UNINDEXED,
    tweet_id UNINDEXED,
    archive_text,
    tokenize = 'unicode61 remove_diacritics 2'
);
INSERT INTO like_fts(rowid, profile_id, tweet_id, archive_text)
SELECT rowid, profile_id, tweet_id, COALESCE(archive_text, '') FROM likes;

CREATE TRIGGER tweets_fts_insert AFTER INSERT ON tweets BEGIN
    INSERT INTO tweet_fts(rowid, tweet_id, text, archive_text, username, display_name)
    VALUES (
        NEW.rowid,
        NEW.id,
        NEW.text,
        '',
        COALESCE((SELECT username FROM users WHERE id = NEW.author_id), ''),
        COALESCE((SELECT display_name FROM users WHERE id = NEW.author_id), '')
    );
END;

CREATE TRIGGER tweets_fts_update AFTER UPDATE OF text, author_id ON tweets BEGIN
    DELETE FROM tweet_fts WHERE rowid = OLD.rowid;
    INSERT INTO tweet_fts(rowid, tweet_id, text, archive_text, username, display_name)
    VALUES (
        NEW.rowid,
        NEW.id,
        NEW.text,
        '',
        COALESCE((SELECT username FROM users WHERE id = NEW.author_id), ''),
        COALESCE((SELECT display_name FROM users WHERE id = NEW.author_id), '')
    );
END;

CREATE TRIGGER tweets_fts_delete AFTER DELETE ON tweets BEGIN
    DELETE FROM tweet_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER likes_fts_insert AFTER INSERT ON likes BEGIN
    INSERT INTO like_fts(rowid, profile_id, tweet_id, archive_text)
    VALUES (NEW.rowid, NEW.profile_id, NEW.tweet_id, COALESCE(NEW.archive_text, ''));
END;

CREATE TRIGGER likes_fts_update AFTER UPDATE OF archive_text, profile_id, tweet_id ON likes BEGIN
    DELETE FROM like_fts WHERE rowid = OLD.rowid;
    INSERT INTO like_fts(rowid, profile_id, tweet_id, archive_text)
    VALUES (NEW.rowid, NEW.profile_id, NEW.tweet_id, COALESCE(NEW.archive_text, ''));
END;

CREATE TRIGGER likes_fts_delete AFTER DELETE ON likes BEGIN
    DELETE FROM like_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER users_fts_insert AFTER INSERT ON users BEGIN
    UPDATE tweet_fts
    SET username = COALESCE(NEW.username, ''), display_name = COALESCE(NEW.display_name, '')
    WHERE rowid IN (SELECT rowid FROM tweets WHERE author_id = NEW.id);
END;

CREATE TRIGGER users_fts_update AFTER UPDATE OF username, display_name ON users BEGIN
    UPDATE tweet_fts
    SET username = COALESCE(NEW.username, ''), display_name = COALESCE(NEW.display_name, '')
    WHERE rowid IN (SELECT rowid FROM tweets WHERE author_id = NEW.id);
END;
