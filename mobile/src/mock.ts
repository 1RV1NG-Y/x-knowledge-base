// Browser-preview stand-in for the Rust side, fed by mobile/.dev/fixtures.json
// (made by scripts/mobile-fixtures.mjs). Never used inside the app.
import type { Playlist, ProfileState, SyncInfo, Tweet, TweetPage, TweetQuery } from '../../src/lib/types';

type Fixture = { profiles: ProfileState; playlists: Playlist[]; tweets: (Tweet & { notes?: Record<number, string> })[] };
let data: Promise<Fixture> | undefined;
const load = (): Promise<Fixture> =>
  (data ??= fetch('/.dev/fixtures.json').then((r) => (r.ok ? r.json() : { profiles: { profiles: [] }, playlists: [], tweets: [] })));

function hash(id: string, seed: number): number {
  let h = seed >>> 0;
  for (const c of id) h = Math.imul(h ^ c.charCodeAt(0), 2654435761) >>> 0;
  return h;
}

export async function mockInvoke(command: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const fx = await load();
  await new Promise((r) => setTimeout(r, 120));
  switch (command) {
    case 'get_profiles':
      return fx.profiles;
    case 'set_active_profile':
      return null;
    case 'get_playlists':
      return fx.playlists;
    case 'get_tweets': {
      const q = args.request as TweetQuery & { seed?: number };
      let list = fx.tweets.slice();
      if (q.playlistId != null) list = list.filter((t) => t.playlistIds.includes(q.playlistId!)).map((t) => ({ ...t, playlistNote: t.notes?.[q.playlistId!] }));
      if (q.search) {
        const words = q.search.toLowerCase().split(/\s+/);
        list = list.filter((t) => words.every((w) => `${t.text} ${t.author?.displayName} ${t.author?.username}`.toLowerCase().includes(w)));
      }
      if (q.author) list = list.filter((t) => t.author?.username?.toLowerCase() === q.author!.toLowerCase());
      if (q.mediaType) list = list.filter((t) => t.media.some((m) => m.kind === q.mediaType));
      const when = (t: Tweet) => t.createdAt ?? t.likedAt ?? '';
      if (q.after) list = list.filter((t) => when(t) >= q.after!);
      if (q.before) list = list.filter((t) => when(t) < q.before!);
      if (q.status) list = list.filter((t) => t.enrichmentStatus === q.status);
      if (q.sortOrder === 'random') list.sort((a, b) => hash(a.id, q.seed ?? 0) - hash(b.id, q.seed ?? 0));
      else list.sort((a, b) => (BigInt(a.id) < BigInt(b.id) ? 1 : -1) * (q.sortOrder === 'oldest' ? -1 : 1));
      return { items: list.slice(q.offset, q.offset + q.limit), total: list.length } satisfies TweetPage;
    }
    case 'sync_status':
      return {
        phone: true,
        status: {
          deviceId: 'mock', deviceName: 'Browser preview', role: 'phone', port: 47130,
          devices: [{ id: 'pc', name: 'Desktop PC', platform: 'linux', connected: true, pairedAt: new Date(Date.now() - 864e5).toISOString(), lastSync: new Date().toISOString() }],
          progress: { phase: 'upToDate', message: 'Up to date', filesDone: 0, filesTotal: 0, bytesDone: 0, bytesTotal: 0, lastSync: new Date(Date.now() - 6e5).toISOString(), posts: fx.profiles.profiles[0]?.postCount },
        },
      } satisfies SyncInfo;
    case 'sync_now':
      return null;
    default:
      throw new Error(`${command} isn't available in the browser preview`);
  }
}
