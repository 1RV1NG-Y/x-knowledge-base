// Browser preview data for the phone UI (`npm run dev:mobile` outside Tauri): a sample of the
// desktop library, read-only, written to mobile/.dev/fixtures.json (gitignored, never bundled).
//
//   node scripts/mobile-fixtures.mjs [path/to/knowledge.db] [count]
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';

const db = process.argv[2] ?? path.join(homedir(), '.local/share/dev.local.xknowledgebase/knowledge.db');
const count = Number(process.argv[3] ?? 400);
const q = (sql) => JSON.parse(execFileSync('sqlite3', ['-readonly', '-json', db, sql], { maxBuffer: 1 << 28 }).toString() || '[]');

const profileRows = q(`SELECT p.id, p.username, p.display_name AS displayName, p.avatar_url AS avatarUrl,
  p.avatar_local_path AS avatarLocalPath, COUNT(l.tweet_id) AS postCount
  FROM profiles p JOIN likes l ON l.profile_id = p.id GROUP BY p.id`);
const active = q(`SELECT value FROM app_settings WHERE key = 'active_profile_id'`)[0]?.value ?? profileRows[0]?.id;
const tweets = q(`SELECT t.id, t.text, l.archive_text AS archiveText, t.created_at AS createdAt, l.liked_at AS likedAt,
  t.canonical_url AS canonicalUrl, t.enrichment_status AS enrichmentStatus,
  u.id AS authorId, u.username, u.display_name AS displayName, u.avatar_url AS avatarUrl, u.avatar_local_path AS avatarLocalPath
  FROM likes l JOIN tweets t ON t.id = l.tweet_id LEFT JOIN users u ON u.id = t.author_id
  WHERE l.profile_id = '${active}' AND t.enrichment_status = 'ok'
  ORDER BY (t.id IN (SELECT tweet_id FROM media)) DESC, random() LIMIT ${count}`);
const ids = tweets.map((t) => `'${t.id}'`).join(',') || "''";
const media = q(`SELECT id, tweet_id AS tweetId, kind, remote_url AS remoteUrl, preview_url AS previewUrl,
  local_path AS localPath, thumbnail_path AS thumbnailPath, width, height, duration_ms AS durationMs
  FROM media WHERE tweet_id IN (${ids}) ORDER BY id`);
const quotes = q(`SELECT tweet_id AS tweetId, quoted_id AS id, text, canonical_url AS canonicalUrl, author_name AS authorName, username
  FROM quotes WHERE tweet_id IN (${ids})`);
const playlists = q(`SELECT p.id, p.name, p.description, p.updated_at AS updatedAt, COUNT(pi.tweet_id) AS itemCount
  FROM playlists p LEFT JOIN playlist_items pi ON pi.playlist_id = p.id WHERE p.profile_id = '${active}' GROUP BY p.id ORDER BY p.updated_at DESC`);
const items = q(`SELECT playlist_id AS playlistId, tweet_id AS tweetId, note FROM playlist_items WHERE tweet_id IN (${ids})`);

const clean = (o) => Object.fromEntries(Object.entries(o).filter(([, v]) => v != null));
const out = tweets.map((t) => clean({
  id: t.id, text: t.text, archiveText: t.archiveText, createdAt: t.createdAt, likedAt: t.likedAt,
  canonicalUrl: t.canonicalUrl, enrichmentStatus: t.enrichmentStatus,
  author: t.authorId ? clean({ id: t.authorId, username: t.username ?? '', displayName: t.displayName ?? '', avatarUrl: t.avatarUrl, avatarLocalPath: t.avatarLocalPath }) : undefined,
  media: media.filter((m) => m.tweetId === t.id).map(({ tweetId, ...m }) => clean(m)),
  quote: (({ tweetId, ...rest } = {}) => (tweetId ? clean(rest) : undefined))(quotes.find((x) => x.tweetId === t.id)),
  playlistIds: items.filter((i) => i.tweetId === t.id).map((i) => i.playlistId),
  notes: Object.fromEntries(items.filter((i) => i.tweetId === t.id && i.note).map((i) => [i.playlistId, i.note])),
}));
const dir = path.resolve(import.meta.dirname, '../mobile/.dev');
mkdirSync(dir, { recursive: true });
writeFileSync(path.join(dir, 'fixtures.json'), JSON.stringify({ profiles: { profiles: profileRows.map(clean), activeProfileId: active }, playlists, tweets: out }));
console.log(`${out.length} posts, ${media.length} media, ${playlists.length} lists → mobile/.dev/fixtures.json`);
