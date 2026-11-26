<script lang="ts">
  import type { Tweet } from '../../src/lib/types';
  import { openUrl } from './api';
  import Avatar from './Avatar.svelte';
  import { fullTime, longDate, postText } from './format';
  import Icon from './Icon.svelte';
  import MediaGrid from './MediaGrid.svelte';
  import Page from './Page.svelte';
  import Quote from './Quote.svelte';
  import RichText from './RichText.svelte';
  import { app } from './state.svelte';

  let { tweet }: { tweet: Tweet } = $props();

  const text = $derived(postText(tweet));
  const name = $derived(tweet.author?.displayName || tweet.author?.username || 'Unknown');
  const lists = $derived(app.playlists.filter((p) => tweet.playlistIds.includes(p.id)));
  const gone = $derived(['unavailable', 'deleted', 'protected', 'suspended'].includes(tweet.enrichmentStatus));

  async function copy() {
    try {
      await navigator.clipboard.writeText(tweet.canonicalUrl);
      app.showToast('Copied to clipboard');
    } catch {
      app.showToast('Couldn’t copy the link');
    }
  }
</script>

<div class="overlay">
  <Page nav={false} autohide={false}>
    {#snippet header()}
      <div class="top">
        <button type="button" class="round" aria-label="Back" onclick={() => app.pop()}><Icon name="back" size={20} /></button>
        <h1>Post</h1>
      </div>
    {/snippet}
    <article>
      <div class="author">
        <button type="button" class="face" aria-label="Liked posts from {name}" disabled={!tweet.author?.username} onclick={() => app.openProfile(tweet.author)}>
          <Avatar path={tweet.author?.avatarLocalPath} url={tweet.author?.avatarUrl} {name} size={40} />
        </button>
        <button type="button" class="who" disabled={!tweet.author?.username} onclick={() => app.openProfile(tweet.author)}>
          <strong>{name}</strong>
          {#if tweet.author?.username}<span>@{tweet.author.username}</span>{/if}
        </button>
        <button type="button" class="round more" aria-label="More" onclick={() => app.push({ kind: 'actions', tweet })}><Icon name="more" size={18} /></button>
      </div>
      {#if text}<div class="text"><RichText {text} large /></div>{/if}
      {#if gone}<p class="notice">This post is no longer available on X.</p>{/if}
      <MediaGrid media={tweet.media} onopen={(index) => app.push({ kind: 'viewer', media: tweet.media, index })} />
      {#if tweet.quote}<Quote quote={tweet.quote} />{/if}
      {#if tweet.createdAt}<p class="time">{fullTime(tweet.createdAt)}</p>{/if}
      <div class="facts">
        {#if tweet.likedAt}<span><Icon name="heart" size={16} /> Liked {longDate(tweet.likedAt)}</span>{/if}
        {#each lists as list (list.id)}
          <button type="button" class="chip" onclick={() => app.push({ kind: 'playlist', playlist: list })}>
            <Icon name="bookmarkFill" size={14} />{list.name}
          </button>
        {/each}
      </div>
      {#if tweet.playlistNote}<p class="note">{tweet.playlistNote}</p>{/if}
      <div class="actions">
        <button type="button" aria-label="Reply on X" onclick={() => void openUrl(tweet.canonicalUrl)}><Icon name="reply" size={22} /></button>
        <span class="liked" aria-label="Liked"><Icon name="heart" size={22} /></span>
        <button type="button" class:listed={lists.length > 0} aria-label="Lists" onclick={() => app.push({ kind: 'actions', tweet })}>
          <Icon name={lists.length ? 'bookmarkFill' : 'bookmark'} size={22} />
        </button>
        <button type="button" aria-label="Copy link" onclick={() => void copy()}><Icon name="share" size={22} /></button>
      </div>
      <button type="button" class="pill outline open" onclick={() => void openUrl(tweet.canonicalUrl)}>
        <Icon name="external" size={18} /> Open on X
      </button>
    </article>
  </Page>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 20; background: var(--bg); }
  .top { display: flex; align-items: center; gap: 24px; height: var(--header-row); padding: 0 8px; }
  h1 { font-size: 20px; font-weight: 700; margin: 0; }
  article { padding: 12px 16px 32px; }
  .author { display: flex; align-items: center; gap: 8px; }
  .face { flex: none; border-radius: 50%; }
  .face:disabled { opacity: 1; }
  .author .who { display: grid; min-width: 0; flex: 1; line-height: 20px; text-align: left; }
  .who:disabled { opacity: 1; }
  .author strong, .author span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .author span { color: var(--muted); }
  .more { color: var(--muted); }
  .text { margin-top: 12px; }
  .notice { color: var(--muted); }
  .time { color: var(--muted); margin: 16px 0; padding-bottom: 16px; border-bottom: 1px solid var(--line); }
  .facts { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; color: var(--muted); }
  .facts span { display: inline-flex; align-items: center; gap: 4px; margin-right: 8px; }
  .facts span :global(svg) { color: var(--like); }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-radius: 9999px;
    border: 1px solid var(--line);
    color: var(--accent);
    font-weight: 700;
    font-size: 14px;
  }
  .note { margin: 12px 0 0; padding: 12px; border-radius: 12px; background: var(--skeleton); }
  .actions {
    display: flex;
    justify-content: space-around;
    margin-top: 12px;
    padding: 6px 0;
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  .actions > * { width: 40px; height: 40px; display: grid; place-items: center; }
  .liked { color: var(--like); }
  .listed { color: var(--accent); }
  .open { margin-top: 20px; width: 100%; }
</style>
