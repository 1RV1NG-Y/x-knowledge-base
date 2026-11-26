<script lang="ts">
  import type { Snippet } from 'svelte';
  import Post from './Post.svelte';
  import type { Feed } from './state.svelte';

  let { feed, empty }: { feed: Feed; empty?: Snippet } = $props();
</script>

{#each feed.items as tweet (tweet.id)}
  <Post {tweet} />
{/each}
{#if feed.loading}
  <div class="spinner" role="status" aria-label="Loading"></div>
{:else if feed.error}
  <div class="empty">
    <p>{feed.error}</p>
    <button type="button" class="pill accent" onclick={() => feed.retry()}>Retry</button>
  </div>
{:else if feed.started && feed.items.length === 0}
  {#if empty}{@render empty()}{/if}
{:else if feed.done && feed.items.length > 0}
  <div class="end" aria-hidden="true"></div>
{/if}

<style>
  .end { height: 64px; }
</style>
