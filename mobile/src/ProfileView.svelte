<script lang="ts">
  import { onMount } from 'svelte';
  import type { Author, SortOrder } from '../../src/lib/types';
  import { openUrl } from './api';
  import Avatar from './Avatar.svelte';
  import FeedList from './FeedList.svelte';
  import Icon from './Icon.svelte';
  import Page from './Page.svelte';
  import Tabs from './Tabs.svelte';
  import { app, Feed, newSeed } from './state.svelte';

  // Everything you liked from one account, like their profile on X but only your likes.
  let { author }: { author: Author } = $props();

  let order = $state<SortOrder>('newest');
  let seed = newSeed();
  let scroller = $state<HTMLElement>();
  const feed = new Feed(() => ({ author: author.username, sortOrder: order, seed }));
  const name = $derived(author.displayName || author.username || 'Unknown');
  const count = $derived(feed.started && !(feed.loading && feed.items.length === 0) ? feed.total : undefined);

  function change(id: string) {
    if (id === order && id !== 'random') {
      scroller?.scrollTo({ top: 0, behavior: 'smooth' });
      return;
    }
    if (id === 'random') seed = newSeed();
    order = id as SortOrder;
    scroller?.scrollTo({ top: 0 });
    void feed.reset();
  }

  async function refresh() {
    if (order === 'random') seed = newSeed();
    await feed.refresh();
  }

  onMount(() => void feed.reset());
</script>

<div class="overlay">
  <Page nav={false} onend={() => void feed.more()} onrefresh={refresh} bind:scroller>
    {#snippet header()}
      <div class="top">
        <button type="button" class="round" aria-label="Back" onclick={() => app.pop()}><Icon name="back" size={20} /></button>
        <div>
          <h1>{name}</h1>
          <span>{count == null ? '…' : `${count.toLocaleString()} ${count === 1 ? 'liked post' : 'liked posts'}`}</span>
        </div>
      </div>
    {/snippet}

    <section class="profile">
      <div class="lead">
        <Avatar path={author.avatarLocalPath} url={author.avatarUrl} {name} size={72} />
        {#if author.username}
          <button type="button" class="pill outline" onclick={() => void openUrl(`https://x.com/${author.username}`)}>
            <Icon name="external" size={16} />View on X
          </button>
        {/if}
      </div>
      <h2>{name}</h2>
      {#if author.username}<p class="handle">@{author.username}</p>{/if}
      <p class="stat">
        {#if count != null}<strong>{count.toLocaleString()}</strong> {count === 1 ? 'post' : 'posts'} you liked{:else}&nbsp;{/if}
      </p>
    </section>

    <div class="tabs">
      <Tabs
        tabs={[{ id: 'newest', label: 'Newest' }, { id: 'oldest', label: 'Oldest' }, { id: 'random', label: 'Shuffled' }]}
        value={order}
        onchange={change}
      />
    </div>

    <FeedList {feed}>
      {#snippet empty()}
        <div class="empty"><h2>No liked posts</h2><p>None of your liked posts are from this account.</p></div>
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
  .profile { padding: 12px 16px 0; }
  .lead { display: flex; align-items: flex-end; justify-content: space-between; gap: 12px; }
  .lead .pill { min-height: 36px; gap: 6px; }
  h2 { margin: 12px 0 0; font-size: 20px; line-height: 24px; font-weight: 800; overflow-wrap: anywhere; }
  .handle { margin: 0; color: var(--muted); }
  .stat { margin: 12px 0; color: var(--muted); font-size: 15px; }
  .stat strong { color: var(--text); }
  .tabs { border-bottom: 1px solid var(--line); }
</style>
