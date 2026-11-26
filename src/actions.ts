// What a post's buttons and menus do, shared by the feed and the post page.
import type { Tweet } from './lib/types';
import { getTweet, openOriginal, retryEnrichment, setPlaylistMembership } from './api';
import { desk, message } from './state.svelte';

export async function toggleList(tweet: Tweet, playlistId: number) {
  const included = !tweet.playlistIds.includes(playlistId);
  try {
    const playlist = await setPlaylistMembership(playlistId, tweet.id, included);
    desk.replacePlaylist(playlist);
    desk.patchTweet({
      ...tweet,
      playlistIds: included ? [...tweet.playlistIds, playlistId] : tweet.playlistIds.filter((id) => id !== playlistId)
    });
    desk.showToast(included ? `Added to ${playlist.name}` : `Removed from ${playlist.name}`);
  } catch (cause) {
    desk.showToast(`Couldn’t change the list: ${message(cause)}`);
  }
}

export async function copyLink(tweet: Tweet) {
  try {
    await navigator.clipboard.writeText(tweet.canonicalUrl);
    desk.showToast('Copied to clipboard');
  } catch {
    desk.showToast('Couldn’t copy the link');
  }
}

export async function openOnX(tweet: Tweet) {
  try {
    await openOriginal(tweet.id);
  } catch (cause) {
    desk.showToast(`Couldn’t open the post: ${message(cause)}`);
  }
}

export async function retry(tweet: Tweet) {
  try {
    await retryEnrichment(tweet.id);
    desk.patchTweet(await getTweet(tweet.id));
    void desk.refreshJob();
    desk.showToast('Fetched the post again');
  } catch (cause) {
    desk.showToast(`Couldn’t fetch the post: ${message(cause)}`);
  }
}

export const isGone = (tweet: Tweet) => ['unavailable', 'deleted', 'protected', 'suspended'].includes(tweet.enrichmentStatus);
