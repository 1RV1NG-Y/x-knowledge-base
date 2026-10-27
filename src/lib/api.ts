import { invoke } from '@tauri-apps/api/core';
import type { ImportSummary, JobStatus, Playlist, ProfileState, Tweet, TweetPage, TweetQuery } from './types';

export function importArchive(path: string): Promise<ImportSummary> {
  return invoke<ImportSummary>('import_archive', { path });
}
export function getProfiles(): Promise<ProfileState> {
  return invoke<ProfileState>('get_profiles');
}

export function setActiveProfile(profileId: string): Promise<void> {
  return invoke<void>('set_active_profile', { profileId });
}

export function getPlaylists(): Promise<Playlist[]> {
  return invoke<Playlist[]>('get_playlists');
}

export function createPlaylist(name: string, description = ''): Promise<Playlist> {
  return invoke<Playlist>('create_playlist', { name, description });
}

export function updatePlaylist(id: number, name: string, description: string): Promise<Playlist> {
  return invoke<Playlist>('update_playlist', { id, name, description });
}

export function deletePlaylist(id: number): Promise<void> {
  return invoke<void>('delete_playlist', { id });
}

export function setPlaylistMembership(playlistId: number, tweetId: string, included: boolean): Promise<Playlist> {
  return invoke<Playlist>('set_playlist_membership', { playlistId, tweetId, included });
}

export function getTweets(request: TweetQuery): Promise<TweetPage> {
  return invoke<TweetPage>('get_tweets', { request });
}

export function getTweet(id: string): Promise<Tweet> {
  return invoke<Tweet>('get_tweet', { id });
}

export function retryEnrichment(id: string): Promise<void> {
  return invoke<void>('retry_enrichment', { id });
}

export function getImportStatus(): Promise<JobStatus> {
  return invoke<JobStatus>('get_import_status');
}

export function loadVideo(mediaId: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('load_video', { mediaId });
}

export function playVideoInMpv(mediaId: string): Promise<void> {
  return invoke<void>('play_video_mpv', { mediaId });
}

export function openOriginal(id: string): Promise<void> {
  return invoke<void>('open_original', { id });
}
