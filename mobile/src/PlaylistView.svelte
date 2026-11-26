<script lang="ts">
  import { onMount } from 'svelte';
  import type { Playlist, SortOrder } from '../../src/lib/types';
  import FeedList from './FeedList.svelte';
  import Icon from './Icon.svelte';
  import Page from './Page.svelte';
  import Tabs from './Tabs.svelte';
  import { app, Feed, newSeed } from './state.svelte';

  let { playlist }: { playlist: Playlist } = $props();

  let order = $state<SortOrder>('random');
  let seed = newSeed();
  let scroller = $state<HTMLElement>();
  const feed = new Feed(() => ({ playlistId: playlist.id, sortOrder: order, seed }));

  function change(id: string) {
    if (id === order && id === 'random') seed = newSeed();
    order = id as SortOrder;
    scroller?.scrollTo({ top: 0 });
    void feed.reset();
  }

  onMount(() => void feed.reset());
</script>

<div class="overlay">
  <Page nav={false} onend={() => void feed.more()} bind:scroller>
    {#snippet header()}
      <div class="top">
        <button type="button" class="round" aria-label="Back" onclick={() => app.pop()}><Icon name="back" size={20} /></button>
        <div>
          <h1>{playlist.name}</h1>
          <span>{playlist.itemCount.toLocaleString()} posts</span>
        </div>
      </div>
      <Tabs tabs={[{ id: 'random', label: 'Shuffled' }, { id: 'newest', label: 'Newest' }, { id: 'oldest', label: 'Oldest' }]} value={order} onchange={change} />
    {/snippet}
    {#if playlist.description}<p class="description">{playlist.description}</p>{/if}
    <FeedList {feed}>
      {#snippet empty()}
        <div class="empty"><h2>Nothing here yet</h2><p>Posts added to this list on the PC show up after the next sync.</p></div>
      {/snippet}
    </FeedList>
  </Page>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 20; background: var(--bg); }
  .top { display: flex; align-items: center; gap: 24px; height: var(--header-row); padding: 0 8px; }
  .top div { display: grid; min-width: 0; line-height: 20px; }
  h1 { font-size: 17px; font-weight: 700; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .top span { color: var(--muted); font-size: 13px; line-height: 16px; }
  .description { margin: 0; padding: 12px 16px; color: var(--muted); border-bottom: 1px solid var(--line); }
</style>
