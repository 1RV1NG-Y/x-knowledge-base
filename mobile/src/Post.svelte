<script lang="ts">
  import type { Tweet } from '../../src/lib/types';
  import { openUrl } from './api';
  import Avatar from './Avatar.svelte';
  import { postText, relativeTime } from './format';
  import Icon from './Icon.svelte';
  import MediaGrid from './MediaGrid.svelte';
  import Quote from './Quote.svelte';
  import RichText from './RichText.svelte';
  import { app } from './state.svelte';

  let { tweet }: { tweet: Tweet } = $props();

  const text = $derived(postText(tweet));
  const gone = $derived(['unavailable', 'deleted', 'protected', 'suspended'].includes(tweet.enrichmentStatus));
  const listed = $derived(tweet.playlistIds.length > 0);
  const name = $derived(tweet.author?.displayName || tweet.author?.username || 'Unknown');

  function stop(event: Event, action: () => void) {
    event.stopPropagation();
    action();
  }

  async function share() {
    try {
      await navigator.clipboard.writeText(tweet.canonicalUrl);
      app.showToast('Copied to clipboard');
    } catch {
      void openUrl(tweet.canonicalUrl);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article class="post" onclick={() => app.push({ kind: 'post', tweet })}>
  {#if tweet.playlistNote}
    <div class="context"><Icon name="bookmarkFill" size={14} /><span>{tweet.playlistNote}</span></div>
  {/if}
  <div class="row">
    <button
      type="button"
      class="face"
      aria-label="Liked posts from {name}"
      disabled={!tweet.author?.username}
      onclick={(e) => stop(e, () => app.openProfile(tweet.author))}
    >
      <Avatar path={tweet.author?.avatarLocalPath} url={tweet.author?.avatarUrl} {name} size={40} />
    </button>
    <div class="body">
      <div class="head">
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="who" onclick={(e) => { if (tweet.author?.username) stop(e, () => app.openProfile(tweet.author)); }}>
          <span class="name">{name}</span>
          {#if tweet.author?.username}<span class="muted handle">@{tweet.author.username}</span>{/if}
        </span>
        {#if tweet.createdAt}<span class="muted">·</span><time class="muted" datetime={tweet.createdAt}>{relativeTime(tweet.createdAt)}</time>{/if}
        <button type="button" class="more" aria-label="More" onclick={(e) => stop(e, () => app.push({ kind: 'actions', tweet }))}>
          <Icon name="more" size={18} />
        </button>
      </div>
      {#if text}<RichText {text} clamp />{/if}
      {#if gone}
        <div class="notice">This post is unavailable.</div>
      {/if}
      <MediaGrid media={tweet.media} onopen={(index) => app.push({ kind: 'viewer', media: tweet.media, index })} />
      {#if tweet.quote}<Quote quote={tweet.quote} />{/if}
      <div class="actions">
        <button type="button" class="action" aria-label="Reply on X" onclick={(e) => stop(e, () => void openUrl(tweet.canonicalUrl))}>
          <span class="halo"><Icon name="reply" size={18.75} /></span>
        </button>
        <span class="action liked" aria-label="Liked"><span class="halo"><Icon name="heart" size={18.75} /></span></span>
        <button type="button" class="action" class:listed aria-label="Lists" onclick={(e) => stop(e, () => app.push({ kind: 'actions', tweet }))}>
          <span class="halo"><Icon name={listed ? 'bookmarkFill' : 'bookmark'} size={18.75} /></span>
        </button>
        <button type="button" class="action" aria-label="Copy link" onclick={(e) => stop(e, () => void share())}>
          <span class="halo"><Icon name="share" size={18.75} /></span>
        </button>
      </div>
    </div>
  </div>
</article>

<style>
  .post {
    padding: 12px 16px 0;
    border-bottom: 1px solid var(--line);
    content-visibility: auto;
    contain-intrinsic-size: auto 320px;
  }
  .post:active { background: var(--hover); }
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
  .row { display: flex; gap: 8px; }
  .face { flex: none; align-self: flex-start; border-radius: 50%; }
  .face:disabled { opacity: 1; }
  .face:not(:disabled):active { filter: brightness(0.85); }
  .who { display: flex; align-items: center; gap: 4px; min-width: 0; flex: 0 1 auto; overflow: hidden; }
  .body { flex: 1; min-width: 0; }
  .head {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    white-space: nowrap;
    height: 20px;
  }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; flex: 0 1 auto; }
  .muted { color: var(--muted); }
  .handle { overflow: hidden; text-overflow: ellipsis; flex: 0 1 auto; min-width: 0; }
  time { flex: none; }
  .more {
    margin-left: auto;
    color: var(--muted);
    width: 34px;
    height: 34px;
    margin-right: -9px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    flex: none;
  }
  .notice {
    margin-top: 12px;
    padding: 12px;
    border-radius: 12px;
    background: var(--skeleton);
    color: var(--muted);
  }
  .actions {
    display: flex;
    justify-content: space-between;
    max-width: 425px;
    margin: 4px 0 0 -8px;
    padding-bottom: 4px;
    color: var(--muted);
  }
  .action { display: flex; align-items: center; height: 36px; }
  .halo { width: 34px; height: 34px; border-radius: 50%; display: grid; place-items: center; }
  .action:active .halo { background: var(--accent-soft); color: var(--accent); }
  .liked { color: var(--like); }
  .listed { color: var(--accent); }
</style>
