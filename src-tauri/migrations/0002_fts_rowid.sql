DROP TRIGGER IF EXISTS tweets_fts_insert;
DROP TRIGGER IF EXISTS tweets_fts_update;
DROP TRIGGER IF EXISTS tweets_fts_delete;
DROP TRIGGER IF EXISTS likes_fts_insert;
DROP TRIGGER IF EXISTS likes_fts_update;
DROP TRIGGER IF EXISTS users_fts_insert;
DROP TRIGGER IF EXISTS users_fts_update;

DELETE FROM tweet_fts;
INSERT INTO tweet_fts(rowid, tweet_id, text, archive_text, username, display_name)
SELECT
    tweets.rowid,
    tweets.id,
    tweets.text,
    COALESCE(likes.archive_text, ''),
    COALESCE(users.username, ''),
    COALESCE(users.display_name, '')
FROM tweets
LEFT JOIN likes ON likes.tweet_id = tweets.id
LEFT JOIN users ON users.id = tweets.author_id;

CREATE TRIGGER tweets_fts_insert AFTER INSERT ON tweets BEGIN
    INSERT INTO tweet_fts(rowid, tweet_id, text, archive_text, username, display_name)
    VALUES (
        NEW.rowid,
        NEW.id,
        NEW.text,
        COALESCE((SELECT archive_text FROM likes WHERE tweet_id = NEW.id), ''),
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
        COALESCE((SELECT archive_text FROM likes WHERE tweet_id = NEW.id), ''),
        COALESCE((SELECT username FROM users WHERE id = NEW.author_id), ''),
        COALESCE((SELECT display_name FROM users WHERE id = NEW.author_id), '')
    );
END;

CREATE TRIGGER tweets_fts_delete AFTER DELETE ON tweets BEGIN
    DELETE FROM tweet_fts WHERE rowid = OLD.rowid;
END;

CREATE TRIGGER likes_fts_insert AFTER INSERT ON likes BEGIN
    UPDATE tweet_fts
    SET archive_text = COALESCE(NEW.archive_text, '')
    WHERE rowid = (SELECT rowid FROM tweets WHERE id = NEW.tweet_id);
END;

CREATE TRIGGER likes_fts_update AFTER UPDATE OF archive_text ON likes BEGIN
    UPDATE tweet_fts
    SET archive_text = COALESCE(NEW.archive_text, '')
    WHERE rowid = (SELECT rowid FROM tweets WHERE id = NEW.tweet_id);
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
