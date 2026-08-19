import { invoke } from '@tauri-apps/api/core';
import type { ImportSummary, JobStatus, ProfileState, Tweet, TweetPage, TweetQuery } from './types';

export function importArchive(path: string): Promise<ImportSummary> {
  return invoke<ImportSummary>('import_archive', { path });
}
export function getProfiles(): Promise<ProfileState> {
  return invoke<ProfileState>('get_profiles');
}

export function setActiveProfile(profileId: string): Promise<void> {
  return invoke<void>('set_active_profile', { profileId });
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
