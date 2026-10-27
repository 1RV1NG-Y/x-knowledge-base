CREATE TABLE playlists (
    id INTEGER PRIMARY KEY,
    profile_id TEXT NOT NULL,
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    description TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (profile_id, name COLLATE NOCASE),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE playlist_items (
    playlist_id INTEGER NOT NULL,
    tweet_id TEXT NOT NULL,
    position INTEGER,
    note TEXT,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    PRIMARY KEY (playlist_id, tweet_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id) ON DELETE CASCADE,
    FOREIGN KEY (tweet_id) REFERENCES tweets(id) ON DELETE CASCADE
);

CREATE INDEX idx_playlist_items_tweet ON playlist_items(tweet_id);

CREATE TRIGGER playlist_items_touch_insert AFTER INSERT ON playlist_items BEGIN
    UPDATE playlists SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = NEW.playlist_id;
END;

CREATE TRIGGER playlist_items_touch_delete AFTER DELETE ON playlist_items BEGIN
    UPDATE playlists SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = OLD.playlist_id;
END;
