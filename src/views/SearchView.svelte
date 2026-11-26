<script lang="ts">
  import type { MediaType } from '../lib/types';
  import Tabs from '../../mobile/src/Tabs.svelte';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import FeedList from '../components/FeedList.svelte';
  import SearchBox from '../components/SearchBox.svelte';
  import { desk, useFeed } from '../state.svelte';

  let { active }: { active: boolean } = $props();

  let media = $state<MediaType | ''>('');
  // "from:handle" narrows to one author; the rest is a full-text search.
  const parsed = $derived.by(() => {
    let author: string | undefined;
    const words = desk.search.trim().split(/\s+/).filter((word) => {
      const match = /^from:@?(\w+)$/i.exec(word);
      if (match) author = match[1];
      return !match && word;
    });
    return { search: words.join(' ') || undefined, author };
  });
  const asking = $derived(Boolean(parsed.search || parsed.author));
  const feed = useFeed(() => ({ ...parsed, mediaType: media || undefined, sortOrder: 'newest' }));

  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const key = JSON.stringify([parsed, media]);
    clearTimeout(timer);
    if (!asking) return;
    timer = setTimeout(() => {
      void key;
      if (active) desk.scrollTop(false);
      void feed.reset();
    }, 250);
  });
</script>

<ColumnHeader>
  <div class="search"><SearchBox autofocus={active && !asking} /></div>
  {#if asking}
    <Tabs
      tabs={[{ id: '', label: 'All' }, { id: 'image', label: 'Photos' }, { id: 'video', label: 'Videos' }]}
      value={media}
      onchange={(id) => (media = id as MediaType | '')}
    />
  {/if}
</ColumnHeader>

{#if asking}
  {#if feed.started && !feed.loading && feed.items.length > 0}
    <p class="count">{feed.total.toLocaleString()} {feed.total === 1 ? 'post' : 'posts'}</p>
  {/if}
  <FeedList {feed}>
    {#snippet empty()}
      <div class="empty">
        <h2>No results{parsed.search ? ` for “${parsed.search}”` : ''}</h2>
        <p>Try other words, or check the spelling.</p>
      </div>
    {/snippet}
  </FeedList>
{:else}
  <div class="empty">
    <h2>Search your likes</h2>
    <p>Words find posts by their text, author or name. Add <b>from:handle</b> to see one account.</p>
  </div>
{/if}

<style>
  .search { padding: 6px 16px 6px; }
  .count { margin: 0; padding: 12px 16px; color: var(--muted); font-size: 13px; border-bottom: 1px solid var(--line); }
</style>
