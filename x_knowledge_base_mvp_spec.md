# X Knowledge Base — MVP Specification

## 1. Purpose

Build a **local-first desktop application for importing, enriching, browsing, and searching a user's X/Twitter archive**, with an initial focus on the user's **liked posts**.

The core problem is that the native X archive is incomplete for knowledge-base use. In particular, liked posts may contain only partial text and limited metadata, while omitting useful information such as:

- author identity
- username / handle
- profile image
- images
- videos
- quoted posts
- reply context
- canonical post URL
- richer post metadata

The MVP should take the sparse archive data, recover the post IDs where possible, enrich each post through external X-compatible metadata endpoints such as **FxTwitter**, cache useful media locally, and expose the result through a **fast, simple, visually clean desktop UI**.

The application is not intended to reproduce all of X. It should feel more like a **local, searchable, media-rich replacement for the Likes page**.

---

## 2. Product Principles

Prioritize these qualities in this order:

1. **Snappy**
   - Search, navigation, scrolling, and opening posts should feel effectively instantaneous.
   - Avoid unnecessary services, background daemons, web servers, or distributed infrastructure.

2. **Simple**
   - Keep the architecture small and understandable.
   - Prefer SQLite and local files over external databases or cloud services.
   - Avoid adding AI, embeddings, RAG, or agents to the MVP.

3. **Efficient**
   - Store structured metadata in SQLite.
   - Store larger media as normal files.
   - Fetch/enrich incrementally and avoid duplicate downloads.

4. **Useful**
   - The app should immediately improve on the X archive by restoring authorship, media, URLs, and searchable metadata.

5. **Visually clean**
   - Good visuals matter, but they are secondary to speed and simplicity.
   - Use a restrained UI rather than an elaborate design system.

---

## 3. MVP Scope

### Required

The MVP must support:

- importing an X/Twitter archive ZIP or extracted archive folder
- locating and parsing the archive's Likes data
- recovering tweet/post IDs and canonical URLs where possible
- enriching each post using FxTwitter or an equivalent public JSON endpoint
- storing normalized post metadata in SQLite
- storing the raw enrichment JSON for future compatibility/debugging
- caching images and thumbnails locally
- optionally caching video files or at minimum storing playable video URLs
- browsing liked posts in a chronological feed
- viewing:
  - post text
  - author name
  - author handle
  - author avatar
  - post date
  - images
  - videos / video thumbnails
  - quoted-post content where available
  - original X URL
- full-text search
- basic filtering
- opening the original post on X
- incremental re-import / enrichment without duplicating records
- graceful handling of deleted, protected, suspended, unavailable, or failed posts

### Nice-to-have for v0.1

These may be included if implementation is cheap:

- local notes on a post
- local tags
- favorite/star inside the local app
- "copy post URL"
- "copy post text"
- reveal cached media in file manager
- enrichment status filter
- retry failed enrichments
- configurable media-download policy

---

## 4. Explicit Non-Goals

Do **not** build these into the MVP:

- generic multi-source knowledge-base architecture
- embeddings
- vector databases
- semantic search
- RAG
- LLM integration
- agent execution
- MCP
- OMP integration
- GitHub ingestion
- YouTube ingestion
- ActivityWatch ingestion
- browser-history ingestion
- cloud sync
- user accounts
- authentication system
- backend web server
- PostgreSQL
- Redis
- Elasticsearch
- Meilisearch
- microservices
- collaborative features
- public sharing
- social features
- full X client functionality

The future possibility that agents may consume this database should **not influence MVP complexity** beyond keeping the local data model reasonably clean and queryable.

---

## 5. Recommended Stack

### Desktop shell

**Tauri 2**

Reasons:

- lightweight compared with Electron
- native desktop packaging
- easy access to filesystem and OS functionality
- frontend can remain standard web UI
- Rust backend is suitable for import, parsing, networking, hashing, media downloads, and SQLite access

### UI

**Svelte + TypeScript**

Reasons:

- small reactive surface area
- good performance
- simple component model
- well suited for feed/search/detail UI
- less framework overhead than a larger SPA stack

### Styling

Prefer:

- normal CSS
- CSS variables
- a small utility layer if useful
- Lucide or similar minimal icon library

Tailwind is acceptable if it speeds up implementation, but a large component framework should not be required.

Avoid building a design-system project inside this project.

### Core

**Rust**

Responsibilities:

- archive detection/parsing
- enrichment HTTP calls
- normalization
- media download/cache
- hashing/deduplication
- SQLite operations
- background job coordination
- filesystem access

### Database

**SQLite**

Use:

- normal relational tables
- SQLite FTS5 for full-text search

No external database.

### Files

Store large binary media on the normal filesystem.

Do not store full image/video blobs inside SQLite unless there is a compelling implementation reason.

---

## 6. High-Level Architecture

```text
X Archive ZIP / Folder
        |
        v
Archive Importer
        |
        v
Sparse Like Records
(tweet/post IDs, text, URLs if present)
        |
        v
Enrichment Queue
        |
        +----> FxTwitter
        |
        +----> fallback enrichment endpoint(s)
        |
        v
Normalizer
        |
        +----> SQLite metadata
        |
        +----> raw JSON snapshot
        |
        +----> local media cache
        |
        v
Desktop UI
  - feed
  - search
  - filters
  - post detail
```

---

## 7. Data Model

Keep the schema compact.

### `tweets`

Suggested fields:

```sql
CREATE TABLE tweets (
    id TEXT PRIMARY KEY,
    author_id TEXT,
    text TEXT NOT NULL DEFAULT '',
    created_at TEXT,
    canonical_url TEXT,
    quoted_tweet_id TEXT,
    reply_to_tweet_id TEXT,
    conversation_id TEXT,
    raw_json TEXT,
    enrichment_status TEXT NOT NULL DEFAULT 'pending',
    enrichment_error TEXT,
    enriched_at TEXT,
    imported_at TEXT NOT NULL,
    FOREIGN KEY(author_id) REFERENCES users(id)
);
```

Possible `enrichment_status` values:

- `pending`
- `ok`
- `deleted`
- `protected`
- `suspended`
- `unavailable`
- `rate_limited`
- `failed`

Do not delete failed/unavailable records. A missing post is itself useful archival information.

### `users`

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT,
    display_name TEXT,
    avatar_url TEXT,
    avatar_local_path TEXT
);
```

### `media`

```sql
CREATE TABLE media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tweet_id TEXT NOT NULL,
    type TEXT NOT NULL,
    remote_url TEXT,
    preview_url TEXT,
    local_path TEXT,
    thumbnail_path TEXT,
    width INTEGER,
    height INTEGER,
    duration_ms INTEGER,
    bitrate INTEGER,
    FOREIGN KEY(tweet_id) REFERENCES tweets(id)
);
```

Suggested `type` values:

- `image`
- `video`
- `gif`
- `unknown`

### `likes`

```sql
CREATE TABLE likes (
    tweet_id TEXT PRIMARY KEY,
    liked_at TEXT,
    archive_text TEXT,
    archive_source TEXT,
    FOREIGN KEY(tweet_id) REFERENCES tweets(id)
);
```

`liked_at` may be unavailable in the archive. It should be nullable.

### Optional `notes`

```sql
CREATE TABLE notes (
    tweet_id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(tweet_id) REFERENCES tweets(id)
);
```

### Optional `tags`

```sql
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE tweet_tags (
    tweet_id TEXT NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY(tweet_id, tag_id),
    FOREIGN KEY(tweet_id) REFERENCES tweets(id),
    FOREIGN KEY(tag_id) REFERENCES tags(id)
);
```

---

## 8. Full-Text Search

Use **SQLite FTS5**.

Index at minimum:

- post text
- author display name
- username
- optional notes

Search should be local and instant after indexing.

Potential UX examples:

```text
rust
karpathy
local llm
from:username
has:image
has:video
before:2026-08-01
after:2026-07-01
```

The MVP does not need a sophisticated parser for every X search operator.

A simple implementation is sufficient:

- plain text terms -> FTS5
- recognized prefixes -> SQL filters
- unrecognized input -> normal FTS query

Suggested initial filters:

- `from:<username>`
- `has:image`
- `has:video`
- `before:YYYY-MM-DD`
- `after:YYYY-MM-DD`
- enrichment status

---

## 9. Archive Import

The importer should accept:

- original X archive ZIP
- extracted X archive directory

The importer should:

1. identify archive structure
2. find Likes-related data files
3. parse all available records
4. recover tweet IDs from:
   - explicit IDs
   - URLs
   - other metadata
5. store the sparse archive record immediately
6. enqueue unresolved/un-enriched posts for enrichment
7. avoid duplicate rows on repeated imports

Preserve whatever original archive text/metadata is available even if enrichment later succeeds.

Never assume the external enrichment service will remain available forever.

---

## 10. Enrichment

### Primary endpoint

Initial practical enrichment provider:

```text
https://api.fxtwitter.com/status/<tweet_id>
```

The provider should be implemented behind a very small interface so another provider can be added later without restructuring the whole app.

Conceptually:

```rust
trait TweetEnricher {
    async fn enrich(&self, tweet_id: &str) -> Result<EnrichedTweet>;
}
```

This is **not** intended as a generic knowledge-source abstraction. It is only an X-post enrichment interface.

### Fallbacks

Potential fallback:

```text
https://cdn.syndication.twimg.com/tweet-result?id=<tweet_id>&token=<value>&lang=en
```

The implementation should tolerate endpoint breakage.

### Raw JSON

Store the complete successful enrichment response in `tweets.raw_json`.

Reasons:

- future parser changes
- missing fields can be recovered later without refetching
- endpoint schema may evolve
- debugging
- migration to another normalized schema later

### Rate handling

Enrichment should be queued rather than fire thousands of requests simultaneously.

Requirements:

- configurable concurrency
- exponential or bounded retry for transient failures
- respect rate-limit responses
- no infinite retry loop
- persist state so application restart does not lose progress

Suggested initial concurrency:

```text
4-8 requests at once
```

Make it configurable internally; no need for elaborate UI initially.

---

## 11. Media Handling

### Images

Prefer downloading images locally.

Store:

- original remote URL
- local path
- dimensions if known

### Avatars

Avatar caching is useful but lower priority than post media.

### Videos

For MVP, support one of these approaches:

#### Preferred

Download the selected useful-quality variant locally.

#### Simpler initial option

Store:

- video metadata
- thumbnail locally
- remote MP4 variant URLs

Then add full video caching shortly afterward.

Do not require video transcoding in the MVP.

### File layout

Suggested:

```text
app-data/
├── knowledge.db
├── media/
│   ├── images/
│   ├── videos/
│   ├── avatars/
│   └── thumbnails/
└── cache/
```

A hash-based layout is acceptable:

```text
media/images/93/93a12c...
```

but not required for the first implementation.

Prevent duplicate downloads by checking:

- tweet/media identity
- existing path
- optionally content hash

---

## 12. UI

The UI should be closer to a **dense local archive browser** than a social network.

### Main layout

Recommended:

```text
┌──────────────────────────────────────────────────────┐
│ Search...                         Filters   Import   │
├──────────────┬───────────────────────────────────────┤
│              │                                       │
│  Navigation  │  Post Feed                            │
│              │                                       │
│  Likes       │  @username · Aug 10                   │
│  Media       │  Post text...                         │
│  Failed      │                                       │
│  Tags*       │  [ image / video ]                    │
│              │                                       │
│              │  quoted post...                       │
│              │                                       │
│              │  Open original                        │
│              │                                       │
├──────────────┴───────────────────────────────────────┤
│ enrichment/import status                            │
└──────────────────────────────────────────────────────┘
```

A two-column layout without permanent navigation is also acceptable if it is simpler.

### Feed card

Each post should display:

- avatar
- display name
- `@username`
- date
- text
- media
- quote preview if available
- status if unavailable/failed
- button/link to original X post

Avoid reproducing X engagement controls unless useful for archival navigation.

### Post detail

Clicking a post may:

- expand inline
- open a side pane
- open a modal

Prefer whichever keeps navigation fastest.

Detail may show:

- full text
- full media
- quoted tweet
- canonical URL
- local notes/tags
- enrichment metadata/status
- cached file actions

### Media gallery

Useful optional view:

```text
All media from liked posts
```

Grid view for images/videos.

This is valuable but should not delay the main feed/search MVP.

---

## 13. Visual Direction

Keep it understated.

Recommended:

- dark/light theme support if easy
- black / white / gray foundation
- one subtle accent color at most
- small-radius cards or nearly flat panels
- strong typography hierarchy
- generous enough spacing for readability
- minimal animation
- instant interactions
- no decorative glassmorphism
- no excessive gradients
- no dashboard-style chart clutter

The app should feel closer to:

- a polished file browser
- a local media library
- a fast native utility

than:

- Notion
- a social dashboard
- a generic SaaS admin template

---

## 14. Performance Requirements

Performance is a primary feature.

### Startup

Application should become interactive quickly even with a large archive.

Do not block startup on:

- media validation
- full re-index
- enrichment
- network calls

### Feed

Use list virtualization once the collection becomes large.

The UI should comfortably handle:

```text
10,000+ liked posts
```

without rendering all cards simultaneously.

### Search

Typical full-text search should feel near-instant.

Target:

```text
<100 ms database/search response for ordinary queries
```

on a normal local database where practical.

### Images

Use thumbnails/previews in feed rather than decoding huge originals unnecessarily.

### Background enrichment

Import/enrichment must not freeze scrolling or search.

---

## 15. Import / Enrichment UX

When archive is imported:

```text
Imported: 8,423 likes
Already enriched: 0
Pending enrichment: 8,423
Failed: 0
```

While enriching:

```text
Enriching posts
3,102 / 8,423
```

The user should be able to browse already imported/enriched content immediately.

Do not force the user to wait for the entire archive before opening the app.

A compact background status area is enough.

---

## 16. Error Handling

The app must expect posts to fail.

Examples:

- deleted post
- protected account
- suspended account
- malformed archive record
- provider outage
- HTTP timeout
- rate limiting
- missing media
- endpoint schema change
- invalid JSON
- unavailable tweet ID

Store the error state.

Display something like:

```text
Post unavailable
Tweet ID: 1234567890

Archive text:
"Original sparse archive text here..."
```

This is preferable to silently dropping the record.

Provide a manual retry action for failed enrichment.

---

## 17. Incremental Behavior

Repeated imports should be safe.

If the same archive or an updated archive is imported:

- do not duplicate likes
- do not duplicate tweets
- update sparse archive metadata if better data exists
- enqueue only missing/failed/stale enrichment as appropriate
- do not redownload media already cached successfully

The database should survive app upgrades.

Use schema migrations from the beginning.

---

## 18. Security / Privacy

This is a local personal archive.

Default behavior:

- data stays local
- no analytics
- no telemetry unless explicitly added later
- no account creation
- no cloud sync
- no upload of archive content to an app-controlled server

External requests should only occur for enrichment/media retrieval.

Make network behavior obvious in code and easy to disable later.

---

## 19. Suggested Rust Modules

Example project organization:

```text
src-tauri/src/
├── main.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   └── models.rs
├── archive/
│   ├── mod.rs
│   └── twitter_archive.rs
├── enrichment/
│   ├── mod.rs
│   ├── fxtwitter.rs
│   └── syndication.rs
├── media/
│   ├── mod.rs
│   └── downloader.rs
├── search/
│   └── mod.rs
└── commands/
    └── mod.rs
```

Frontend:

```text
src/
├── lib/
│   ├── components/
│   │   ├── TweetCard.svelte
│   │   ├── MediaGrid.svelte
│   │   ├── SearchBar.svelte
│   │   ├── FilterBar.svelte
│   │   └── ImportStatus.svelte
│   ├── stores/
│   └── api/
├── routes/
└── App.svelte
```

Do not over-engineer this structure if fewer files are cleaner.

---

## 20. Suggested Tauri Commands

Potential frontend/backend API:

```text
import_archive(path)

get_tweets(
    search,
    filters,
    offset,
    limit
)

get_tweet(id)

get_media(tweet_id)

retry_enrichment(tweet_id)

get_import_status()

get_enrichment_status()

open_original(tweet_id)

save_note(tweet_id, text)          # optional

set_tags(tweet_id, tags)           # optional
```

Prefer pagination/cursoring rather than returning the entire archive.

---

## 21. Development Phases

### Phase 1 — Archive parser

Goal:

- open archive
- extract liked post IDs
- show imported IDs/text in a basic list
- persist to SQLite

No enrichment required yet.

### Phase 2 — FxTwitter enrichment

Goal:

- enrich one post
- normalize response
- persist author/text/date/media URLs/raw JSON
- handle failure status

Then make it work as a background queue.

### Phase 3 — Feed UI

Goal:

- scroll through enriched likes
- display text/authors/images
- open original
- handle failed/unavailable posts

### Phase 4 — Search

Goal:

- FTS5 index
- instant text search
- basic `from:` / media / date filters

### Phase 5 — Media cache

Goal:

- local image cache
- thumbnails
- optional video caching
- deduplication

### Phase 6 — Polish

Goal:

- import status
- retry failures
- virtualized feed
- settings for media caching
- notes/tags if desired
- packaging

---

## 22. MVP Acceptance Criteria

The MVP is successful when all of the following work:

1. User selects an X archive ZIP or folder.
2. Application finds and imports the Likes data.
3. Imported liked posts are represented in SQLite.
4. Post IDs are enriched through FxTwitter where possible.
5. Enriched records show:
   - author
   - handle
   - date
   - text
   - images/video metadata
   - canonical URL
6. Images appear directly in the local feed.
7. Failed/deleted posts remain visible as unavailable records.
8. Re-importing does not duplicate data.
9. User can search post text locally.
10. User can filter at least by:
    - author
    - image/video presence
    - date
11. User can open the original X post.
12. A large collection remains responsive.
13. The app does not require:
    - login
    - cloud backend
    - external database
    - AI service
14. All durable KB data remains usable locally without the enrichment provider after it has been cached.

---

## 23. Future Ideas — Do Not Implement Yet

These are intentional future possibilities, not MVP requirements.

### Better X reconstruction

- threads
- replies
- quote chains
- bookmarks
- user's own posts
- following/followers archive data
- list memberships
- richer author history

### Search improvements

- saved searches
- collections
- better query parser
- semantic search
- embeddings

### Knowledge-base features

- automatic tags
- link extraction
- GitHub repo metadata
- article extraction
- OCR on images
- transcript extraction from videos

### Agentic use

The local X database could later become input to an external agentic system.

Example future command:

> Look at the interesting technologies I liked today, research the linked repositories, and build a useful prototype from the best ideas.

This future agentic functionality should be implemented as a **consumer of the database**, not embedded into the initial X archive application.

The MVP only needs to preserve enough structured information for future tools to query it cleanly.

---

## 24. Guiding Rule for the Coding Agent

When choosing between:

```text
more architecture
```

and:

```text
a smaller implementation that already imports,
enriches, searches, and displays the archive quickly
```

choose the smaller implementation.

The core product is:

> **A fast, local, searchable, media-rich X Likes archive.**

Everything else can be added after that works well.
