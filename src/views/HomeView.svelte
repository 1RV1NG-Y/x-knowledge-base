<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import Tabs from '../../mobile/src/Tabs.svelte';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import FeedList from '../components/FeedList.svelte';
  import LatestOptions from '../components/LatestOptions.svelte';
  import { defaultLatest, desk, filterCount, newSeed, useFeed } from '../state.svelte';

  // "For you": your likes shuffled (a new shuffle on launch, on the shuffle button or on
  // clicking the tab again). "Latest": newest first, or what its options say.
  let seed = newSeed();
  const forYou = useFeed(() => ({ sortOrder: 'random', seed }));
  const latest = useFeed(() => {
    const o = desk.latest;
    return {
      sortOrder: o.sort,
      mediaType: o.media || undefined,
      author: o.author || undefined,
      after: o.after || undefined,
      before: o.before || undefined,
      status: o.status || undefined
    };
  });
  const filters = $derived(filterCount(desk.latest));
  let tabsEl = $state<HTMLElement>();
  let optionsAnchor = $state<HTMLElement>();

  export function shuffle() {
    seed = newSeed();
    desk.scrollTop(false);
    void forYou.refresh();
  }

  function choose(id: string) {
    if (id === desk.homeTab) {
      if (id === 'random') shuffle();
      else optionsAnchor = optionsAnchor ? undefined : (tabsEl?.querySelector<HTMLElement>('[data-tab="newest"]') ?? undefined);
      return;
    }
    desk.homeTab = id as 'random' | 'newest';
    const feed = id === 'random' ? forYou : latest;
    if (!feed.started) void feed.reset();
  }

  // New Latest options: start over from the top.
  let applied = JSON.stringify(desk.latest);
  $effect(() => {
    const key = JSON.stringify(desk.latest);
    if (key === applied) return;
    applied = key;
    if (!latest.started) return;
    if (desk.homeTab === 'newest') desk.scrollTop(false);
    void latest.reset();
  });

  // The library changed (account switch, import): reload what's been opened.
  let version = desk.libraryVersion;
  $effect(() => {
    if (desk.libraryVersion === version) return;
    version = desk.libraryVersion;
    seed = newSeed();
    void forYou.reset();
    if (latest.started) void latest.reset();
  });

  onMount(() => {
    void (desk.homeTab === 'random' ? forYou : latest).reset();
  });
</script>

<ColumnHeader>
  <div class="tabs" bind:this={tabsEl}>
    <Tabs
      tabs={[
        { id: 'random', label: 'For you' },
        { id: 'newest', label: desk.latest.sort === 'oldest' ? 'Oldest' : 'Latest', menu: true, badge: filters > 0 }
      ]}
      value={desk.homeTab}
      onchange={choose}
    />
    {#if desk.homeTab === 'random'}
      <button type="button" class="round shuffle" aria-label="Shuffle again" title="Shuffle again" onclick={shuffle}><Icon name="shuffle" size={20} /></button>
    {:else}
      <button
        type="button"
        class="round shuffle"
        class:on={filters > 0}
        aria-label="Sort and filter"
        title="Sort and filter"
        onclick={(e) => (optionsAnchor = optionsAnchor ? undefined : e.currentTarget)}
      ><Icon name="sliders" size={20} /></button>
    {/if}
  </div>
</ColumnHeader>

<div class:hidden={desk.homeTab !== 'random'}>
  <FeedList feed={forYou}>
    {#snippet empty()}
      <div class="empty">
        <h2>Welcome to your likes</h2>
        <p>Import your X archive (the ZIP from X, or its extracted folder) to bring your liked posts here. Importing again is safe.</p>
      </div>
    {/snippet}
  </FeedList>
</div>
<div class:hidden={desk.homeTab !== 'newest'}>
  <FeedList feed={latest}>
    {#snippet empty()}
      <div class="empty">
        {#if filters > 0}
          <h2>Nothing matches</h2>
          <p>No liked posts fit these filters.</p>
          <button type="button" class="pill accent" onclick={() => desk.setLatest({ ...defaultLatest, sort: desk.latest.sort })}>Clear filters</button>
        {:else}
          <h2>No liked posts yet</h2>
          <p>Import your X archive to bring your likes here.</p>
        {/if}
      </div>
    {/snippet}
  </FeedList>
</div>

{#if optionsAnchor}
  <LatestOptions anchor={optionsAnchor} onclose={() => (optionsAnchor = undefined)} />
{/if}

<style>
  .tabs { position: relative; padding-right: 52px; }
  .shuffle { position: absolute; right: 10px; top: 50%; transform: translateY(-50%); color: var(--text); }
  .shuffle.on { color: var(--accent); }
  .hidden { display: none; }
</style>
