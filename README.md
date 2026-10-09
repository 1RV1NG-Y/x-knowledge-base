# X Knowledge Base

A local-first desktop app that turns your X/Twitter archive into a fast, searchable, media-rich library of your liked posts.

The official X archive is sparse: your likes come with partial text and little else — no author, no images or video, no quoted posts. X Knowledge Base imports the archive, fills in each post through the public [FxTwitter](https://github.com/FixTweet/FxTwitter) API, caches the media locally, and gives you a quick feed you can scroll, search, and filter. No accounts, no server, no cloud.

Built with Tauri 2, Svelte 5, TypeScript, Rust, and SQLite (FTS5).

## Features

- **Import** an archive ZIP or an extracted archive folder. The app finds `data/like.js` (and your account/profile info) and recovers post IDs and URLs. Re-importing updates in place without duplicates.
- **Enrichment** fetches author name, handle, avatar, full text, images, videos, and quoted posts in the background, with bounded concurrency and backoff on rate limits. Deleted, protected, or unavailable posts are marked rather than dropped, and failed ones can be retried.
- **Local media cache** for images, thumbnails, and avatars, stored as normal files.
- **Feed** with newest-first / oldest-first ordering, a zoomable image viewer, and in-app video playback (or open the video in `mpv` if installed).
- **Full-text search** across post text and authors.
- **Filters** by author, media type, date range, and enrichment status.
- **Profiles** — import archives from more than one account and switch between them.
- **Open original** to jump to the post on X.

## Requirements

- Node.js 20+ and npm
- Rust 1.77+ and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform
- Optional: [mpv](https://mpv.io) for external video playback

## Run

```sh
npm install
npm run tauri dev
```

Build a release bundle with:

```sh
npm run tauri build
```

## Get your archive

On X: **Settings → Your account → Download an archive of your data**. When it's ready, download the ZIP and import it directly — there's no need to extract it.

## Data

Everything lives in the app's data directory (`~/.local/share/dev.local.xknowledgebase/` on Linux): `knowledge.db` (SQLite) with the normalized posts, the raw enrichment JSON for each one, and a `media/` folder with cached images, thumbnails, and avatars. Your archive files are only read, never modified.

Archive exports (`twitter-*/`, `twitter-*.zip`) and local databases are gitignored, so keeping them inside the project folder is safe.

## Project layout

```
src/                    Svelte UI (feed, search, filters, profile switcher)
src-tauri/src/archive   Archive parsing (ZIP or folder)
src-tauri/src/enrichment  FxTwitter client and enrichment queue
src-tauri/src/media     Local media cache
src-tauri/src/db        SQLite storage and full-text search
src-tauri/migrations    Database schema
```
