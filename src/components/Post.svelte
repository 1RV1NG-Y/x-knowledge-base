<script lang="ts">
  import type { Tweet } from '../lib/types';
  import Avatar from '../../mobile/src/Avatar.svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import Quote from '../../mobile/src/Quote.svelte';
  import RichText from '../../mobile/src/RichText.svelte';
  import { fullTime, postText, relativeTime } from '../../mobile/src/format';
  import { openUrl } from '../api';
  import { copyLink, isGone, openOnX, retry } from '../actions';
  import { desk } from '../state.svelte';
  import ListMenu from './ListMenu.svelte';
  import MediaGrid from './MediaGrid.svelte';
  import Menu from './Menu.svelte';

  let { tweet }: { tweet: Tweet } = $props();

  const text = $derived(postText(tweet));
  const gone = $derived(isGone(tweet));
  const listed = $derived(tweet.playlistIds.length > 0);
  const name = $derived(tweet.author?.displayName || tweet.author?.username || 'Unknown');
  const author = $derived(tweet.author?.username ? tweet.author : undefined);

  let moreButton = $state<HTMLButtonElement>();
  let listButton = $state<HTMLButtonElement>();
  let menu = $state<'more' | 'lists'>();
  let retrying = $state(false);

  function stop(event: Event, action: () => void) {
    event.stopPropagation();
    action();
  }

  function openPost(event: MouseEvent) {
    // Selecting text shouldn't count as a click.
    if (window.getSelection()?.toString()) return;
    if ((event.target as HTMLElement).closest('a, button, video')) return;
    desk.open({ kind: 'post', tweet });
  }

  async function doRetry() {
    menu = undefined;
    retrying = true;
    await retry(tweet);
    retrying = false;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article class="post" onclick={openPost}>
  {#if tweet.playlistNote}
    <div class="context"><Icon name="bookmarkFill" size={14} /><span>{tweet.playlistNote}</span></div>
  {/if}
  <div class="row">
    <button type="button" class="face" aria-label="Liked posts from {name}" title={author ? `Liked posts from @${author.username}` : undefined} disabled={!author} onclick={(e) => stop(e, () => desk.openProfile(author))}>
      <Avatar path={tweet.author?.avatarLocalPath} url={tweet.author?.avatarUrl} {name} size={40} />
    </button>
    <div class="body">
      <div class="head">
        <button type="button" class="who" disabled={!author} onclick={(e) => stop(e, () => desk.openProfile(author))}>
          <span class="name">{name}</span>
          {#if author}<span class="muted handle">@{author.username}</span>{/if}
        </button>
        {#if tweet.createdAt}
          <span class="muted">·</span>
          <time class="muted" datetime={tweet.createdAt} title={fullTime(tweet.createdAt)}>{relativeTime(tweet.createdAt)}</time>
        {/if}
        {#if tweet.enrichmentStatus === 'pending'}<span class="tag pending">Pending</span>{/if}
        {#if tweet.enrichmentStatus === 'failed'}<span class="tag failed">Failed</span>{/if}
        <button type="button" class="more" aria-label="More" aria-expanded={menu === 'more'} bind:this={moreButton} onclick={(e) => stop(e, () => (menu = menu === 'more' ? undefined : 'more'))}>
          <Icon name="more" size={18.75} />
        </button>
      </div>
      {#if text}<RichText {text} clamp />{/if}
      {#if gone}
        <div class="notice">This post is unavailable.{#if tweet.enrichmentError} <span>{tweet.enrichmentError}</span>{/if}</div>
      {:else if tweet.enrichmentStatus === 'failed'}
        <div class="notice">
          Couldn’t fetch this post from X.{#if tweet.enrichmentError} <span>{tweet.enrichmentError}</span>{/if}
          <button type="button" class="link" disabled={retrying} onclick={(e) => stop(e, () => void doRetry())}>{retrying ? 'Trying…' : 'Try again'}</button>
        </div>
      {/if}
      <MediaGrid media={tweet.media} />
      {#if tweet.quote}<Quote quote={tweet.quote} />{/if}
      <div class="actions">
        <button type="button" class="action reply" aria-label="Reply on X" title="Reply on X" onclick={(e) => stop(e, () => void openUrl(tweet.canonicalUrl))}>
          <span class="halo"><Icon name="reply" size={18.75} /></span>
        </button>
        <span class="action liked" title="Liked"><span class="halo"><Icon name="heart" size={18.75} /></span></span>
        <button type="button" class="action save" class:listed aria-label="Lists" title="Save to a list" bind:this={listButton} onclick={(e) => stop(e, () => (menu = menu === 'lists' ? undefined : 'lists'))}>
          <span class="halo"><Icon name={listed ? 'bookmarkFill' : 'bookmark'} size={18.75} /></span>
          {#if listed}<span class="count">{tweet.playlistIds.length}</span>{/if}
        </button>
        <button type="button" class="action share" aria-label="Copy link" title="Copy link" onclick={(e) => stop(e, () => void copyLink(tweet))}>
          <span class="halo"><Icon name="share" size={18.75} /></span>
        </button>
      </div>
    </div>
  </div>
</article>

{#if menu === 'more' && moreButton}
  <Menu anchor={moreButton} onclose={() => (menu = undefined)} label="Post actions">
    <button type="button" class="item" onclick={() => { menu = undefined; void openOnX(tweet); }}><Icon name="external" size={18} />Open on X</button>
    <button type="button" class="item" onclick={() => { menu = undefined; void copyLink(tweet); }}><Icon name="link" size={18} />Copy link</button>
    {#if author}
      <button type="button" class="item" onclick={() => { menu = undefined; desk.openProfile(author); }}><Icon name="user" size={18} />Liked posts from @{author.username}</button>
    {/if}
    <button type="button" class="item" onclick={() => { menu = undefined; desk.open({ kind: 'post', tweet }); }}><Icon name="info" size={18} />Post details</button>
    {#if tweet.enrichmentStatus !== 'ok'}
      <button type="button" class="item" onclick={() => void doRetry()}><Icon name="retry" size={18} />Fetch from X again</button>
    {/if}
  </Menu>
{/if}
{#if menu === 'lists' && listButton}
  <ListMenu {tweet} anchor={listButton} onclose={() => (menu = undefined)} />
{/if}

<style>
  .post {
    padding: 12px 16px 0;
    border-bottom: 1px solid var(--line);
    cursor: pointer;
    transition: background-color 120ms ease;
  }
  .post:hover { background: var(--hover); }
  .context {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: -4px 0 4px 28px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 700;
    line-height: 16px;
  }
  .context span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row { display: flex; gap: 12px; }
  .face { flex: none; align-self: flex-start; border-radius: 50%; transition: filter 120ms ease; }
  .face:not(:disabled):hover { filter: brightness(0.88); }
  .body { flex: 1; min-width: 0; }
  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    white-space: nowrap;
    height: 20px;
  }
  .who { display: flex; align-items: center; gap: 4px; min-width: 0; flex: 0 1 auto; overflow: hidden; cursor: pointer; }
  .who:disabled { cursor: inherit; }
  .who:not(:disabled):hover .name { text-decoration: underline; }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; flex: 0 1 auto; }
  .muted { color: var(--muted); }
  .handle { overflow: hidden; text-overflow: ellipsis; flex: 0 1 auto; min-width: 0; }
  time { flex: none; }
  time:hover { text-decoration: underline; }
  .tag { flex: none; margin-left: 4px; padding: 0 6px; border-radius: 4px; font-size: 12px; font-weight: 700; line-height: 18px; }
  .tag.pending { color: var(--warning); background: rgb(255 173 31 / 0.12); }
  .tag.failed { color: var(--danger); background: var(--danger-soft); }
  .more {
    margin-left: auto;
    color: var(--muted);
    width: 34px;
    height: 34px;
    margin-right: -8px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    flex: none;
    transition: background-color 120ms ease, color 120ms ease;
  }
  .more:hover, .more[aria-expanded='true'] { background: var(--accent-soft); color: var(--accent); }
  .notice {
    margin-top: 12px;
    padding: 12px;
    border-radius: 12px;
    background: var(--raised);
    color: var(--muted);
  }
  .notice span { display: block; font-size: 13px; margin-top: 2px; overflow-wrap: anywhere; }
  .link { color: var(--accent); font-weight: 700; margin-top: 6px; }
  .link:hover { text-decoration: underline; }
  .actions {
    display: flex;
    justify-content: space-between;
    max-width: 425px;
    margin: 4px 0 0 -8px;
    padding-bottom: 4px;
    color: var(--muted);
  }
  .action { display: flex; align-items: center; height: 36px; font-size: 13px; transition: color 120ms ease; }
  .halo { width: 34px; height: 34px; border-radius: 50%; display: grid; place-items: center; transition: background-color 120ms ease; }
  .reply:hover, .share:hover, .save:hover { color: var(--accent); }
  .reply:hover .halo, .share:hover .halo, .save:hover .halo { background: var(--accent-soft); }
  .liked { color: var(--like); cursor: default; }
  .listed { color: var(--accent); }
  .count { margin-left: 2px; }
</style>
