<script lang="ts">
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { Tweet } from '../types';
  import MediaGrid from './MediaGrid.svelte';

  interface Props {
    tweet: Tweet;
    expanded?: boolean;
    detailBusy?: boolean;
    actionBusy?: boolean;
    ondetail: (id: string) => void;
    onretry: (id: string) => void;
    onopen: (id: string) => void;
  }

  let { tweet, expanded = false, detailBusy = false, actionBusy = false, ondetail, onretry, onopen }: Props = $props();

  const dateFormatter = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' });

  function formatDate(value?: string): string {
    if (!value) return 'Date unknown';
    const date = new Date(value);
    return Number.isNaN(date.valueOf()) ? value : dateFormatter.format(date);
  }

  function avatarSource(): string | undefined {
    if (tweet.author?.avatarLocalPath) return convertFileSrc(tweet.author.avatarLocalPath);
    return tweet.author?.avatarUrl;
  }

  function fallbackAvatar(event: Event): void {
    const image = event.currentTarget as HTMLImageElement;
    const fallback = tweet.author?.avatarUrl;
    if (fallback && image.src !== fallback) image.src = fallback;
    else image.hidden = true;
  }

  const failed = $derived(tweet.enrichmentStatus === 'failed');
  const unavailable = $derived(['unavailable', 'deleted', 'protected', 'suspended'].includes(tweet.enrichmentStatus));
  const pending = $derived(tweet.enrichmentStatus === 'pending');
</script>

<article class:problem={failed || unavailable} aria-labelledby={`tweet-${tweet.id}-author`}>
  <div class="avatar" aria-hidden="true">
    {#if avatarSource()}
      <img src={avatarSource()} alt="" loading="eager" decoding="async" onerror={fallbackAvatar} />
    {:else}
      <span>{(tweet.author?.displayName ?? '?').slice(0, 1).toUpperCase()}</span>
    {/if}
  </div>

  <div class="tweet-body">
    <header>
      <div class="identity" id={`tweet-${tweet.id}-author`}>
        <strong>{tweet.author?.displayName ?? 'Unknown author'}</strong>
        {#if tweet.author?.username}<span>@{tweet.author.username}</span>{/if}
        <span aria-hidden="true">·</span>
        <time datetime={tweet.createdAt ?? tweet.likedAt}>{formatDate(tweet.createdAt ?? tweet.likedAt)}</time>
      </div>
      {#if pending}<span class="status pending">Pending</span>{/if}
      {#if failed}<span class="status failed">Failed</span>{/if}
      {#if unavailable}<span class="status unavailable">Unavailable</span>{/if}
    </header>

    {#if failed || unavailable}
      <div class="notice" role="status">
        <strong>{unavailable ? 'This post is no longer available.' : 'Post enrichment failed.'}</strong>
        {#if tweet.enrichmentError}<span>{tweet.enrichmentError}</span>{/if}
      </div>
    {/if}

    {#if tweet.text}
      <p class="tweet-text">{tweet.text}</p>
    {:else if tweet.archiveText}
      <p class="tweet-text archive-text">{tweet.archiveText}</p>
    {:else if !failed && !unavailable}
      <p class="tweet-text archive-text">Post text has not been recovered yet.</p>
    {/if}

    <MediaGrid media={tweet.media} />

    {#if tweet.quote}
      <blockquote class="quote">
        <div>
          <strong>{tweet.quote.authorName ?? 'Quoted post'}</strong>
          {#if tweet.quote.username}<span>@{tweet.quote.username}</span>{/if}
        </div>
        <p>{tweet.quote.text}</p>
      </blockquote>
    {/if}

    {#if expanded}
      <div class="details">
        <span>Post ID <code>{tweet.id}</code></span>
        <span>Liked {formatDate(tweet.likedAt)}</span>
        <span>Status: {tweet.enrichmentStatus}</span>
      </div>
    {/if}

    <footer>
      <button type="button" class="text-button" onclick={() => ondetail(tweet.id)} disabled={detailBusy} aria-expanded={expanded}>
        {detailBusy ? 'Loading…' : expanded ? 'Less detail' : 'Details'}
      </button>
      {#if failed}
        <button type="button" class="text-button retry" onclick={() => onretry(tweet.id)} disabled={actionBusy}>
          {actionBusy ? 'Retrying…' : 'Retry enrichment'}
        </button>
      {/if}
      <button type="button" class="text-button open" onclick={() => onopen(tweet.id)} disabled={actionBusy}>
        Open original <span aria-hidden="true">↗</span>
      </button>
    </footer>
  </div>
</article>
