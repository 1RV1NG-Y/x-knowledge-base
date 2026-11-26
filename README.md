# X Knowledge Base

A local-first desktop app that turns your X/Twitter archive into a fast, searchable, media-rich library of your liked posts.

The official X archive is sparse: your likes come with partial text and little else — no author, no images or video, no quoted posts. X Knowledge Base imports the archive, fills in each post through the public [FxTwitter](https://github.com/FixTweet/FxTwitter) API, caches the media locally, and gives you a quick feed you can scroll, search, and filter. No accounts, no server, no cloud.

Built with Tauri 2, Svelte 5, TypeScript, Rust, and SQLite (FTS5).

## Features

- **Import** an archive ZIP or an extracted archive folder. The app finds `data/like.js` (and your account/profile info) and recovers post IDs and URLs. Re-importing updates in place without duplicates.
- **Enrichment** fetches author name, handle, avatar, full text, images, videos, and quoted posts in the background, with bounded concurrency and backoff on rate limits. Deleted, protected, or unavailable posts are marked rather than dropped, and failed ones can be retried.
- **Local media cache** for images, thumbnails, and avatars, stored as normal files.
- **Laid out like X**: navigation on the left, the feed in the middle, search, your library and lists on the right. The app draws its own title bar; the window still moves, snaps and resizes like any other.
- **Feed** with two tabs: **For you** (your likes shuffled; click the tab or the shuffle button for a new order) and **Latest** (newest or oldest first, with filters by media type, date range, author and enrichment status). A zoomable image viewer, and in-app video playback (or open the video in `mpv` if installed).
- **Full-text search** across post text and authors; `from:username` narrows to one account.
- **Profiles of the people you like**: click an avatar or name to see every liked post from that account.
- **Post pages** with the dates, the lists it's in, its enrichment status and the text from your archive.
- **Lists** (playlists): group liked posts by topic. Save posts with the bookmark button, or let an agent build lists (see below).
- **Accounts**: import archives from more than one account and switch between them.
- **Open on X** to jump to the original post.
- Keyboard: `Esc` (or the mouse's back button) goes back, `/` searches.

## Phone app (Android)

A read-only copy of the library for your phone, laid out like the X mobile app: **For you**
(your likes in a random order, reshuffled on each launch or with the shuffle button),
**Latest**, search (`from:username` narrows to one author), lists, a post page, and a zoomable
image viewer. Videos stream from X, so they need an internet connection; everything else works
offline.

**Sync is one-way, PC → phone**, over your local network, with no account or server (the same
approach as reader-sync and music-stream: QUIC via [iroh](https://github.com/n0-computer/iroh),
mDNS discovery, pairing by QR code). The PC stays the source of truth; nothing on the phone is
sent back.

1. On the PC, click **Phone** in the left column, then **Pair a phone**. It shows a QR code and a text code (single use,
   10 minutes).
2. On the phone, open the app's **Sync** tab and scan the code (or paste the text code).
3. The phone downloads a slimmed copy of the database (no raw enrichment JSON) plus the cached
   images and avatars, then keeps itself up to date whenever the desktop app is open and both
   are on the same Wi-Fi: when the app comes to the foreground, on **Sync now**, and every few
   minutes while connected. Only changed files are fetched; interrupted downloads resume.

The devices talk over UDP port **47130** and mDNS (UDP 5353). Allow them in your firewall, e.g.
`sudo ufw allow from 192.168.1.0/24 to any port 47130 proto udp`.

Build and install (Android SDK + NDK, Java 21; the APK is signed with your local debug key):

```sh
npm run android:build                                  # → android-dist/x-knowledge-base.apk
adb install -r android-dist/x-knowledge-base.apk
```

`npm run dev:mobile` serves the phone UI in a browser; run `node scripts/mobile-fixtures.mjs`
first to give it a sample of your library to show (written to the gitignored `mobile/.dev/`).

## Requirements

- Node.js 20+ and npm
- Rust 1.77+ and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
- Optional: [mpv](https://mpv.io) for external video playback

## Run

```sh
npm install
npm run tauri dev
```

`npm run dev` serves the desktop UI in a browser too, answering from the same sample fixtures as the phone preview (`node scripts/mobile-fixtures.mjs`).

Build a release bundle with:

```sh
npm run tauri build
```

## Get your archive

On X: **Settings → Your account → Download an archive of your data**. When it's ready, download the ZIP and import it directly — there's no need to extract it.

## Data

Everything lives in the app's data directory (`~/.local/share/dev.local.xknowledgebase/` on Linux): `knowledge.db` (SQLite) with the normalized posts, the raw enrichment JSON for each one, and a `media/` folder with cached images, thumbnails, and avatars. Your archive files are only read, never modified.

Archive exports (`twitter-*/`, `twitter-*.zip`) and local databases are gitignored, so keeping them inside the project folder is safe.

## Building playlists with an agent

Playlists are plain SQLite tables in `knowledge.db`, so an agent can search your likes and create playlists directly. Changes show up the next time the app window gets focus.

```sql
-- playlists: one per topic, owned by a profile; names are unique per profile (case-insensitive)
playlists(id INTEGER PRIMARY KEY, profile_id TEXT, name TEXT, description TEXT DEFAULT '',
          created_at TEXT, updated_at TEXT)
-- playlist_items: posts in a playlist, shown newest or oldest first like the main feed
playlist_items(playlist_id INTEGER, tweet_id TEXT, note TEXT, added_at TEXT,
               PRIMARY KEY (playlist_id, tweet_id))
```

`created_at`, `updated_at` and `added_at` have defaults, and `note` is shown on the post inside the playlist. Run `PRAGMA foreign_keys = ON;` first so deleting a playlist removes its items.

```sql
-- Which profile? (the app shows the one in app_settings.active_profile_id)
SELECT id, username FROM profiles;

-- Search one profile's likes: enriched text and authors, archive text, and quoted posts
SELECT t.id, u.username, t.created_at, substr(t.text, 1, 280) AS text
FROM likes l
JOIN tweets t ON t.id = l.tweet_id
LEFT JOIN users u ON u.id = t.author_id
WHERE l.profile_id = '<profile id>'
  AND (t.rowid IN (SELECT rowid FROM tweet_fts WHERE tweet_fts MATCH 'rust OR cargo')
       OR l.rowid IN (SELECT rowid FROM like_fts WHERE like_fts MATCH 'rust OR cargo')
       OR t.id IN (SELECT tweet_id FROM quotes WHERE quotes.text LIKE '%rust%'));

-- Create a playlist and add the posts you picked
INSERT INTO playlists(profile_id, name, description)
VALUES ('<profile id>', 'Rust', 'Learning Rust and its tooling') RETURNING id;
INSERT OR IGNORE INTO playlist_items(playlist_id, tweet_id, note)
VALUES (<playlist id>, '<tweet id>', 'Best intro thread');
```

Only posts the profile has liked appear in its playlists. The database is the source of truth, so keep backups before bulk edits.

## Project layout

```
src/                    Svelte UI of the desktop app (shares components with mobile/)
mobile/                 Svelte UI of the phone app (built into dist-mobile/ for Android)
src-tauri/src/sync      PC → phone sync: pairing, snapshots, transfers
src-tauri/src/archive   Archive parsing (ZIP or folder)
src-tauri/src/enrichment  FxTwitter client and enrichment queue
src-tauri/src/media     Local media cache
src-tauri/src/db        SQLite storage and full-text search
src-tauri/migrations    Database schema
```
