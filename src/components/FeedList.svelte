<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Feed } from '../state.svelte';
  import Post from './Post.svelte';

  // Posts of one feed; the next page loads well before the end comes into view.
  let { feed, empty }: { feed: Feed; empty?: Snippet } = $props();
  let sentinel = $state<HTMLElement>();

  $effect(() => {
    if (!sentinel) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) void feed.more();
    }, { rootMargin: '0px 0px 1800px 0px' });
    observer.observe(sentinel);
    return () => observer.disconnect();
  });
</script>

{#each feed.items as tweet (tweet.id)}
  <Post {tweet} />
{/each}
{#if feed.loading}
  <div class="spinner" role="status" aria-label="Loading"></div>
{:else if feed.error}
  <div class="empty">
    <h2>Couldn’t load posts</h2>
    <p>{feed.error}</p>
    <button type="button" class="pill accent" onclick={() => feed.retry()}>Retry</button>
  </div>
{:else if feed.started && feed.items.length === 0}
  {#if empty}{@render empty()}{/if}
{:else if feed.done && feed.items.length > 0}
  <div class="end" aria-hidden="true"></div>
{/if}
<div class="sentinel" bind:this={sentinel}></div>

<style>
  .end { height: 50vh; }
  .sentinel { height: 1px; }
</style>
