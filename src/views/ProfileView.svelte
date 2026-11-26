<script lang="ts">
  import { onMount } from 'svelte';
  import type { Author, SortOrder } from '../lib/types';
  import Avatar from '../../mobile/src/Avatar.svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import Tabs from '../../mobile/src/Tabs.svelte';
  import { openUrl } from '../api';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import FeedList from '../components/FeedList.svelte';
  import { desk, newSeed, useFeed } from '../state.svelte';

  // Everything you liked from one account: their profile on X, but only your likes.
  let { author }: { author: Author } = $props();

  let order = $state<SortOrder>('newest');
  let seed = newSeed();
  const feed = useFeed(() => ({ author: author.username, sortOrder: order, seed }));
  const name = $derived(author.displayName || author.username);
  const count = $derived(feed.started && !(feed.loading && feed.items.length === 0) ? feed.total : undefined);
  const hue = $derived([...author.username].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));

  function change(id: string) {
    if (id === order && id !== 'random') return desk.scrollTop();
    if (id === 'random') seed = newSeed();
    order = id as SortOrder;
    desk.scrollTop(false);
    void feed.reset();
  }

  onMount(() => void feed.reset());
</script>

<ColumnHeader back title={name} subtitle={count == null ? '…' : `${count.toLocaleString()} ${count === 1 ? 'liked post' : 'liked posts'}`} />

<section class="profile">
  <div class="banner" style:background="linear-gradient(120deg, hsl({hue} 45% 32%), hsl({hue + 50} 50% 22%))"></div>
  <div class="lead">
    <span class="ring"><Avatar path={author.avatarLocalPath} url={author.avatarUrl} {name} size={134} /></span>
    <button type="button" class="pill outline" onclick={() => void openUrl(`https://x.com/${author.username}`)}>
      <Icon name="external" size={16} />View on X
    </button>
  </div>
  <h2>{name}</h2>
  <p class="handle">@{author.username}</p>
  <p class="stat">
    {#if count != null}<strong>{count.toLocaleString()}</strong> {count === 1 ? 'post' : 'posts'} you liked{:else}&nbsp;{/if}
  </p>
</section>

<div class="tabs">
  <Tabs tabs={[{ id: 'newest', label: 'Newest' }, { id: 'oldest', label: 'Oldest' }, { id: 'random', label: 'Shuffled' }]} value={order} onchange={change} />
</div>

<FeedList {feed}>
  {#snippet empty()}
    <div class="empty"><h2>No liked posts</h2><p>None of your liked posts are from this account.</p></div>
  {/snippet}
</FeedList>

<style>
  .banner { height: 200px; }
  .profile { padding-bottom: 4px; }
  .lead { display: flex; align-items: flex-end; justify-content: space-between; padding: 0 16px; margin-top: -68px; }
  .ring { border-radius: 50%; border: 4px solid var(--bg); background: var(--bg); line-height: 0; }
  .lead .pill { margin-bottom: 4px; }
  h2 { margin: 12px 16px 0; font-size: 20px; line-height: 24px; font-weight: 800; overflow-wrap: anywhere; }
  .handle { margin: 0 16px; color: var(--muted); }
  .stat { margin: 12px 16px; color: var(--muted); }
  .stat strong { color: var(--text); }
  .tabs { border-bottom: 1px solid var(--line); }
</style>
