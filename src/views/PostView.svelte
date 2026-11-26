<script lang="ts">
  import { onMount } from 'svelte';
  import type { Tweet } from '../lib/types';
  import Avatar from '../../mobile/src/Avatar.svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import Quote from '../../mobile/src/Quote.svelte';
  import RichText from '../../mobile/src/RichText.svelte';
  import { fullTime, longDate, postText } from '../../mobile/src/format';
  import { copyLink, isGone, openOnX, retry } from '../actions';
  import { getTweet, openUrl } from '../api';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import ListMenu from '../components/ListMenu.svelte';
  import MediaGrid from '../components/MediaGrid.svelte';
  import { desk } from '../state.svelte';

  // One post on its own, with everything the archive knows about it.
  let { tweet }: { tweet: Tweet } = $props();

  const text = $derived(postText(tweet));
  const name = $derived(tweet.author?.displayName || tweet.author?.username || 'Unknown');
  const author = $derived(tweet.author?.username ? tweet.author : undefined);
  const lists = $derived(desk.playlists.filter((p) => tweet.playlistIds.includes(p.id)));
  const gone = $derived(isGone(tweet));
  const archiveDiffers = $derived(Boolean(tweet.archiveText && tweet.text && tweet.archiveText.trim() !== tweet.text.trim()));
  const statusLabel: Record<string, string> = {
    ok: 'Fetched from X',
    pending: 'Waiting to be fetched from X',
    failed: 'Couldn’t be fetched from X',
    unavailable: 'No longer on X',
    deleted: 'Deleted on X',
    protected: 'Protected account',
    suspended: 'Account suspended'
  };
  let listButton = $state<HTMLButtonElement>();
  let listsOpen = $state(false);
  let retrying = $state(false);

  async function doRetry() {
    retrying = true;
    await retry(tweet);
    retrying = false;
  }

  // The feed's copy can be trimmed; load the full one.
  onMount(() => {
    void getTweet(tweet.id).then((full) => desk.patchTweet(full)).catch(() => {});
  });
</script>

<ColumnHeader back title="Post" />

<article>
  <div class="author">
    <button type="button" class="face" aria-label="Liked posts from {name}" disabled={!author} onclick={() => desk.openProfile(author)}>
      <Avatar path={tweet.author?.avatarLocalPath} url={tweet.author?.avatarUrl} {name} size={40} />
    </button>
    <button type="button" class="who" disabled={!author} onclick={() => desk.openProfile(author)}>
      <strong>{name}</strong>
      {#if author}<span>@{author.username}</span>{/if}
    </button>
    <button type="button" class="pill outline small" onclick={() => void openOnX(tweet)}><Icon name="external" size={16} />Open on X</button>
  </div>
  {#if text}<div class="text"><RichText {text} large /></div>{/if}
  {#if gone}<p class="notice">This post is no longer available on X.</p>{/if}
  <MediaGrid media={tweet.media} />
  {#if tweet.quote}<Quote quote={tweet.quote} />{/if}
  {#if tweet.createdAt}<p class="time">{fullTime(tweet.createdAt)}</p>{/if}

  <div class="actions">
    <button type="button" class="reply" aria-label="Reply on X" title="Reply on X" onclick={() => void openUrl(tweet.canonicalUrl)}><Icon name="reply" size={22} /></button>
    <span class="liked" title="Liked"><Icon name="heart" size={22} /></span>
    <button type="button" class="save" class:listed={lists.length > 0} aria-label="Lists" title="Save to a list" bind:this={listButton} onclick={() => (listsOpen = !listsOpen)}>
      <Icon name={lists.length ? 'bookmarkFill' : 'bookmark'} size={22} />
    </button>
    <button type="button" class="share" aria-label="Copy link" title="Copy link" onclick={() => void copyLink(tweet)}><Icon name="share" size={22} /></button>
  </div>

  {#if lists.length}
    <div class="chips">
      {#each lists as list (list.id)}
        <button type="button" class="chip" onclick={() => desk.open({ kind: 'list', playlist: list })}><Icon name="bookmarkFill" size={14} />{list.name}</button>
      {/each}
    </div>
  {/if}
  {#if tweet.playlistNote}<p class="note">{tweet.playlistNote}</p>{/if}

  <dl class="facts">
    {#if tweet.likedAt}<div><dt>Liked</dt><dd>{longDate(tweet.likedAt)}</dd></div>{/if}
    <div>
      <dt>Status</dt>
      <dd class:bad={tweet.enrichmentStatus !== 'ok'}>{statusLabel[tweet.enrichmentStatus] ?? tweet.enrichmentStatus}</dd>
    </div>
    {#if tweet.enrichmentError}<div><dt>Error</dt><dd class="bad">{tweet.enrichmentError}</dd></div>{/if}
    <div><dt>Post ID</dt><dd><code>{tweet.id}</code></dd></div>
  </dl>
  {#if tweet.enrichmentStatus !== 'ok'}
    <button type="button" class="pill outline small" disabled={retrying} onclick={() => void doRetry()}>
      <Icon name="retry" size={16} />{retrying ? 'Fetching…' : 'Fetch from X again'}
    </button>
  {/if}
  {#if archiveDiffers}
    <details class="archive">
      <summary>Text in your X archive</summary>
      <p>{tweet.archiveText}</p>
    </details>
  {/if}
</article>

{#if listsOpen && listButton}
  <ListMenu {tweet} anchor={listButton} onclose={() => (listsOpen = false)} />
{/if}

<style>
  article { padding: 12px 16px 64px; }
  .author { display: flex; align-items: center; gap: 12px; }
  .face { flex: none; border-radius: 50%; }
  .face:not(:disabled):hover { filter: brightness(0.88); }
  .who { display: grid; min-width: 0; flex: 1; line-height: 20px; text-align: left; }
  .who strong, .who span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who span { color: var(--muted); }
  .who:not(:disabled):hover strong { text-decoration: underline; }
  .text { margin-top: 12px; }
  .notice { margin: 12px 0 0; padding: 12px; border-radius: 12px; background: var(--raised); color: var(--muted); }
  .time { margin: 16px 0 0; padding-bottom: 16px; color: var(--muted); border-bottom: 1px solid var(--line); }
  .actions { display: flex; justify-content: space-around; padding: 4px 0; border-bottom: 1px solid var(--line); color: var(--muted); }
  .actions > * { width: 44px; height: 44px; border-radius: 50%; display: grid; place-items: center; transition: background-color 120ms ease, color 120ms ease; }
  .reply:hover, .share:hover, .save:hover { color: var(--accent); background: var(--accent-soft); }
  .liked { color: var(--like); }
  .listed { color: var(--accent); }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; padding-top: 12px; }
  .chip { display: inline-flex; align-items: center; gap: 6px; padding: 4px 12px; border-radius: 9999px; background: var(--accent-soft); color: var(--accent); font-weight: 700; font-size: 14px; }
  .chip:hover { filter: brightness(1.2); }
  .note { margin: 12px 0 0; padding: 12px; border-left: 3px solid var(--accent); background: var(--raised); border-radius: 0 8px 8px 0; white-space: pre-wrap; }
  .facts { margin: 16px 0; display: grid; gap: 8px; }
  .facts div { display: flex; gap: 12px; }
  dt { color: var(--muted); width: 72px; flex: none; }
  dd { margin: 0; overflow-wrap: anywhere; }
  dd.bad { color: var(--danger); }
  code { font-size: 13px; user-select: all; }
  .archive { margin-top: 16px; color: var(--muted); }
  .archive summary { cursor: pointer; font-weight: 700; }
  .archive p { white-space: pre-wrap; color: var(--text); }
</style>
