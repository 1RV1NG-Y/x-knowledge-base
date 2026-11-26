// The desktop app's calls into the Rust side. In the browser preview they are answered by
// mock.ts from the phone preview's fixtures.
import { invoke as tauriInvoke, isTauri } from '@tauri-apps/api/core';
import type {
  ImportSummary, JobStatus, Playlist, ProfileState, SyncInfo, SyncInvite, Tweet, TweetPage, TweetQuery
} from './lib/types';

export const inTauri = isTauri();
export { fileUrl, openUrl } from '../mobile/src/api';

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(command, args);
  const { mockInvoke } = await import('./mock');
  return mockInvoke(command, args) as Promise<T>;
}

export const importArchive = (path: string) => invoke<ImportSummary>('import_archive', { path });
export const getImportStatus = () => invoke<JobStatus>('get_import_status');
export const getProfiles = () => invoke<ProfileState>('get_profiles');
export const setActiveProfile = (profileId: string) => invoke<void>('set_active_profile', { profileId });
export const getPlaylists = () => invoke<Playlist[]>('get_playlists');
export const createPlaylist = (name: string, description = '') => invoke<Playlist>('create_playlist', { name, description });
export const updatePlaylist = (id: number, name: string, description: string) =>
  invoke<Playlist>('update_playlist', { id, name, description });
export const deletePlaylist = (id: number) => invoke<void>('delete_playlist', { id });
export const setPlaylistMembership = (playlistId: number, tweetId: string, included: boolean) =>
  invoke<Playlist>('set_playlist_membership', { playlistId, tweetId, included });
export const getTweets = (request: TweetQuery) => invoke<TweetPage>('get_tweets', { request });
export const getTweet = (id: string) => invoke<Tweet>('get_tweet', { id });
export const retryEnrichment = (id: string) => invoke<void>('retry_enrichment', { id });
export const loadVideo = (mediaId: string) => invoke<ArrayBuffer>('load_video', { mediaId });
export const playVideoInMpv = (mediaId: string) => invoke<void>('play_video_mpv', { mediaId });
export const openOriginal = (id: string) => invoke<void>('open_original', { id });
export const getSyncStatus = () => invoke<SyncInfo>('sync_status');
export const createSyncInvite = () => invoke<SyncInvite>('sync_invite');
export const removeSyncDevice = (id: string) => invoke<void>('sync_remove_device', { id });

/** Folder or ZIP picker for an archive import. */
export async function pickArchive(directory: boolean): Promise<string | undefined> {
  if (!inTauri) return undefined;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({
    directory,
    multiple: false,
    title: directory ? 'Choose extracted X archive folder' : 'Choose X archive ZIP',
    filters: directory ? undefined : [{ name: 'ZIP archive', extensions: ['zip'] }]
  });
  return typeof selected === 'string' ? selected : undefined;
}

export async function onEvent(name: string, handler: () => void): Promise<() => void> {
  if (!inTauri) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen(name, handler);
}
