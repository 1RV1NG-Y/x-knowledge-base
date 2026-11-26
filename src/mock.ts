// Browser-preview stand-in for the desktop's Rust side: the phone preview's fixtures plus
// in-memory answers for the desktop-only commands. Never used inside the app.
import type { JobStatus, Playlist, SyncInfo, SyncInvite, Tweet } from './lib/types';
import { mockInvoke as phoneMock } from '../mobile/src/mock';

let playlists: Playlist[] | undefined;
const now = () => new Date().toISOString();

async function lists(): Promise<Playlist[]> {
  return (playlists ??= (await phoneMock('get_playlists')) as Playlist[]);
}

export async function mockInvoke(command: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (command) {
    case 'get_playlists':
      return [...(await lists())];
    case 'create_playlist': {
      const created: Playlist = { id: Date.now(), name: String(args.name), description: String(args.description ?? ''), itemCount: 0, updatedAt: now() };
      playlists = [created, ...(await lists())];
      return created;
    }
    case 'update_playlist': {
      const all = await lists();
      const found = all.find((p) => p.id === args.id)!;
      Object.assign(found, { name: args.name, description: args.description, updatedAt: now() });
      return { ...found };
    }
    case 'delete_playlist':
      playlists = (await lists()).filter((p) => p.id !== args.id);
      return null;
    case 'set_playlist_membership': {
      const found = (await lists()).find((p) => p.id === args.playlistId)!;
      found.itemCount += args.included ? 1 : -1;
      return { ...found };
    }
    case 'get_tweet': {
      const page = (await phoneMock('get_tweets', { request: { sortOrder: 'newest', offset: 0, limit: 100000 } })) as { items: Tweet[] };
      const found = page.items.find((t) => t.id === args.id);
      if (!found) throw new Error('Post not found');
      return found;
    }
    case 'get_import_status':
      return { phase: '', processed: 0, total: 0, failed: 0, message: '' } satisfies JobStatus;
    case 'sync_status': {
      const phone = (await phoneMock('sync_status')) as SyncInfo;
      return {
        phone: false,
        status: phone.status && {
          ...phone.status,
          role: 'pc',
          deviceName: 'This PC',
          devices: [{ id: 'phone', name: 'Galaxy S22', platform: 'android', connected: false, pairedAt: new Date(Date.now() - 864e5).toISOString(), lastSync: new Date(Date.now() - 36e5).toISOString() }],
        },
      } satisfies SyncInfo;
    }
    case 'sync_invite':
      return { ticket: 'KBX1PREVIEWONLYCODE', expiresAt: new Date(Date.now() + 6e5).toISOString(), qrSvg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#fff"/><path d="M1 1h3v3H1zM6 1h3v3H6zM1 6h3v3H1zM6 6h1v1H6zM8 8h1v1H8z"/></svg>' } satisfies SyncInvite;
    case 'retry_enrichment':
    case 'sync_remove_device':
    case 'open_original':
    case 'set_active_profile':
      return null;
    default:
      return phoneMock(command, args);
  }
}
