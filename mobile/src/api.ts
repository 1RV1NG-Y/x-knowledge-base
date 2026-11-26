// The phone app's calls into the Rust side. Outside Tauri (the browser preview used during
// development) the same calls are answered from local fixtures; see mock.ts.
import { convertFileSrc, invoke as tauriInvoke, isTauri } from '@tauri-apps/api/core';
import type { Playlist, ProfileState, SyncInfo, TweetPage, TweetQuery } from '../../src/lib/types';

export const inTauri = isTauri();

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(command, args);
  const { mockInvoke } = await import('./mock');
  return mockInvoke(command, args) as Promise<T>;
}

export const getTweets = (request: TweetQuery) => invoke<TweetPage>('get_tweets', { request });
export const getProfiles = () => invoke<ProfileState>('get_profiles');
export const setActiveProfile = (profileId: string) => invoke<void>('set_active_profile', { profileId });
export const getPlaylists = () => invoke<Playlist[]>('get_playlists');
export const getSyncStatus = () => invoke<SyncInfo>('sync_status');
export const useSyncCode = (code: string) => invoke<string>('sync_use_code', { code });
export const removeSyncDevice = (id: string) => invoke<void>('sync_remove_device', { id });
export const syncNow = () => invoke<void>('sync_now');

/** A file in the phone's library as an image or video URL. */
export function fileUrl(path?: string): string | undefined {
  if (!path) return undefined;
  return inTauri ? convertFileSrc(path) : `/@fs${path}`;
}

export async function onEvent(name: string, handler: () => void): Promise<() => void> {
  if (!inTauri) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen(name, handler);
}

export async function openUrl(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, '_blank');
    return;
  }
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(url);
}

/** Android Back button. `handler` returns true when it handled the press; otherwise the app closes. */
export function onBackButton(handler: () => boolean): () => void {
  if (!inTauri || !/Android/.test(navigator.userAgent)) return () => {};
  const listener = import('@tauri-apps/api/app').then(({ onBackButtonPress, exit }) =>
    onBackButtonPress(() => {
      if (!handler()) void exit(0);
    })
  );
  return () => void listener.then((l) => l.unregister());
}

/** Scans a QR code with the camera (Android). */
export async function scanCode(): Promise<string | undefined> {
  const scanner = await import('@tauri-apps/plugin-barcode-scanner');
  let permission = await scanner.checkPermissions();
  if (permission !== 'granted') permission = await scanner.requestPermissions();
  if (permission !== 'granted') throw new Error('Camera permission is needed to scan the code.');
  const result = await scanner.scan({ formats: [scanner.Format.QRCode], windowed: false });
  return result?.content;
}
