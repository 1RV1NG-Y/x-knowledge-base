<script lang="ts">
  import { onMount } from 'svelte';
  import ActionsSheet from './ActionsSheet.svelte';
  import Avatar from './Avatar.svelte';
  import Drawer from './Drawer.svelte';
  import FeedList from './FeedList.svelte';
  import Icon from './Icon.svelte';
  import LatestOptions from './LatestOptions.svelte';
  import Lists from './Lists.svelte';
  import Page from './Page.svelte';
  import PlaylistView from './PlaylistView.svelte';
  import PostDetail from './PostDetail.svelte';
  import ProfileView from './ProfileView.svelte';
  import Search from './Search.svelte';
  import SyncPage from './SyncPage.svelte';
  import Tabs from './Tabs.svelte';
  import Viewer from './Viewer.svelte';
  import { onBackButton, onEvent, syncNow } from './api';
  import { app, defaultLatest, Feed, filterCount, newSeed, type Tab } from './state.svelte';

  // Home: "For you" is your likes in a random order (a new shuffle on each launch, on
  // the shuffle button or on pull to refresh); "Latest" is newest first, or whatever its
  // options sheet (tap the tab again) says.
  let homeTab = $state<'random' | 'newest'>('random');
  let seed = newSeed();
  const forYou = new Feed(() => ({ sortOrder: 'random', seed }));
  const latest = new Feed(() => {
    const o = app.latest;
    return {
      sortOrder: o.sort,
      mediaType: o.media || undefined,
      author: o.author || undefined,
      after: o.after || undefined,
      before: o.before || undefined,
      status: o.status || undefined,
    };
  });
  const filters = $derived(filterCount(app.latest));
  const latestLabel = $derived(app.latest.sort === 'oldest' ? 'Oldest' : 'Latest');

  // New options: start the Latest feed over from the top.
  let appliedOptions = JSON.stringify(app.latest);
  $effect(() => {
    const key = JSON.stringify(app.latest);
    if (key === appliedOptions) return;
    appliedOptions = key;
    if (!latest.started) return;
    scrollers.newest?.scrollTo({ top: 0 });
    void latest.reset();
  });
  const feed = $derived(homeTab === 'random' ? forYou : latest);
  let scrollers = $state<Record<string, HTMLElement | undefined>>({});
  let chromeHidden = $state(false);
  let seenVersion = 0;
  const stale = $derived(app.libraryVersion !== seenVersion && feed.items.length > 0);

  const progress = $derived(app.sync?.status?.progress);
  const syncing = $derived(progress && ['checking', 'downloading', 'applying'].includes(progress.phase));
  const unpaired = $derived(app.sync?.status && app.sync.status.devices.length === 0);

  function scrollTop(id: string) {
    scrollers[id]?.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function shuffle() {
    seed = newSeed();
    scrollers[homeTab]?.scrollTo({ top: 0 });
    void forYou.reset();
  }

  // Pull to refresh: a new shuffle for For you, a fresh first page for Latest.
  async function pullRefresh(id: string) {
    seenVersion = app.libraryVersion;
    if (id === 'random') {
      seed = newSeed();
      await forYou.refresh();
    } else {
      await latest.refresh();
    }
  }

  function refreshFeeds() {
    seenVersion = app.libraryVersion;
    scrollers.random?.scrollTo({ top: 0 });
    scrollers.newest?.scrollTo({ top: 0 });
    void forYou.reset();
    if (latest.started) void latest.reset();
  }

  function chooseHomeTab(id: string) {
    if (id === homeTab) {
      if (id === 'newest') app.push({ kind: 'latestOptions' });
      else scrollTop(id);
      return;
    }
    homeTab = id as 'random' | 'newest';
    if (!feed.started) void feed.reset();
  }

  function navigate(tab: Tab) {
    if (tab === app.tab) {
      if (tab === 'home') {
        const scroller = scrollers[homeTab];
        if (scroller && scroller.scrollTop > 0) scrollTop(homeTab);
        else if (homeTab === 'random') shuffle();
      }
      return;
    }
    app.tab = tab;
  }

  onMount(() => {
    void app.refreshLibrary();
    void app.refreshSync();
    void forYou.reset();
    const stops = [
      onEvent('sync-status', () => void app.refreshSync()),
      onEvent('library-synced', () => {
        void app.refreshLibrary();
        app.libraryVersion += 1;
        if (forYou.items.length === 0) refreshFeeds();
        else app.showToast('Library updated from your PC');
      }),
    ];
    const stopBack = onBackButton(() => {
      if (app.pop()) return true;
      if (app.tab !== 'home') {
        app.tab = 'home';
        return true;
      }
      return false;
    });
    // Back in the foreground: the network may have changed, and the PC may have news.
    const visible = () => {
      if (document.visibilityState === 'visible') void syncNow().catch(() => {});
    };
    document.addEventListener('visibilitychange', visible);
    return () => {
      stops.forEach((stop) => void stop.then((f) => f()));
      stopBack();
      document.removeEventListener('visibilitychange', visible);
    };
  });
</script>

<div class="shell">
  {#each ['random', 'newest'] as id (id)}
    {@const list = id === 'random' ? forYou : latest}
    <Page
      active={app.tab === 'home' && homeTab === id}
      onchrome={(hidden) => (chromeHidden = hidden)}
      onend={() => void list.more()}
      onrefresh={() => pullRefresh(id)}
      bind:scroller={scrollers[id]}
    >
      {#snippet header()}
        <div class="top">
          <button type="button" class="me" aria-label="Account and lists" onclick={() => app.push({ kind: 'drawer' })}>
            <Avatar path={app.activeProfile?.avatarLocalPath} url={app.activeProfile?.avatarUrl} name={app.activeProfile?.displayName ?? '?'} size={32} />
          </button>
          <button type="button" class="mark" aria-label="Back to top" onclick={() => scrollTop(id)}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <rect x="5.5" y="3.5" width="12" height="15" rx="2.5" transform="rotate(-12 11.5 11)" opacity="0.45" />
              <rect x="6.5" y="5" width="12" height="15" rx="2.5" />
            </svg>
          </button>
          <span class="status">
            {#if syncing}<span class="mini-spinner" role="status" aria-label="Syncing"></span>{/if}
          </span>
        </div>
        <Tabs
          tabs={[{ id: 'random', label: 'For you' }, { id: 'newest', label: latestLabel, menu: true, badge: filters > 0 }]}
          value={homeTab}
          onchange={chooseHomeTab}
        />
      {/snippet}
      <FeedList feed={list}>
        {#snippet empty()}
          <div class="empty">
            {#if id === 'newest' && filters > 0}
              <h2>Nothing matches</h2>
              <p>No liked posts fit these filters.</p>
              <div class="row">
                <button type="button" class="pill outline" onclick={() => app.push({ kind: 'latestOptions' })}>Change filters</button>
                <button type="button" class="pill accent" onclick={() => app.setLatest({ ...defaultLatest, sort: app.latest.sort })}>Clear filters</button>
              </div>
            {:else if unpaired || !app.profiles.profiles.length}
              <h2>Your likes live on your PC</h2>
              <p>Pair this phone with X Knowledge Base on your PC to bring over your liked posts, lists and saved images.</p>
              <button type="button" class="pill accent" onclick={() => (app.tab = 'sync')}>Sync with PC</button>
            {:else}
              <h2>No liked posts here</h2>
              <p>This account has no liked posts yet.</p>
            {/if}
          </div>
        {/snippet}
      </FeedList>
    </Page>
  {/each}

  <Search active={app.tab === 'search'} onchrome={(hidden) => (chromeHidden = hidden)} />
  <Lists active={app.tab === 'lists'} onchrome={(hidden) => (chromeHidden = hidden)} />
  <SyncPage active={app.tab === 'sync'} onchrome={(hidden) => (chromeHidden = hidden)} />

  {#if stale && app.tab === 'home'}
    <button type="button" class="fresh" class:raised={chromeHidden} onclick={refreshFeeds}>↑ Library updated</button>
  {/if}

  {#if app.tab === 'home' && homeTab === 'random' && forYou.items.length > 0}
    <button type="button" class="fab" class:faded={chromeHidden} aria-label="Shuffle again" onclick={shuffle}>
      <Icon name="shuffle" size={24} />
    </button>
  {/if}

  <nav class="tabbar" class:faded={chromeHidden} aria-label="Sections">
    <button type="button" aria-label="Home" aria-current={app.tab === 'home' ? 'page' : undefined} onclick={() => navigate('home')}>
      <Icon name={app.tab === 'home' ? 'homeFill' : 'home'} size={26} />
    </button>
    <button type="button" aria-label="Search" aria-current={app.tab === 'search' ? 'page' : undefined} onclick={() => navigate('search')}>
      <Icon name={app.tab === 'search' ? 'searchBold' : 'search'} size={26} />
    </button>
    <button type="button" aria-label="Lists" aria-current={app.tab === 'lists' ? 'page' : undefined} onclick={() => navigate('lists')}>
      <Icon name={app.tab === 'lists' ? 'listsBold' : 'lists'} size={26} />
    </button>
    <button type="button" aria-label="Sync with PC" aria-current={app.tab === 'sync' ? 'page' : undefined} onclick={() => navigate('sync')}>
      <Icon name={app.tab === 'sync' ? 'laptopBold' : 'laptop'} size={26} />
      {#if syncing}<span class="dot busy"></span>{:else if unpaired || progress?.phase === 'error'}<span class="dot alert"></span>{/if}
    </button>
  </nav>

  {#each app.overlays as overlay, i (i)}
    {#if overlay.kind === 'post'}<PostDetail tweet={overlay.tweet} />
    {:else if overlay.kind === 'playlist'}<PlaylistView playlist={overlay.playlist} />
    {:else if overlay.kind === 'viewer'}<Viewer media={overlay.media} index={overlay.index} />
    {:else if overlay.kind === 'drawer'}<Drawer onswitched={refreshFeeds} />
    {:else if overlay.kind === 'actions'}<ActionsSheet tweet={overlay.tweet} />
    {:else if overlay.kind === 'latestOptions'}<LatestOptions />
    {:else if overlay.kind === 'profile'}<ProfileView author={overlay.author} />
    {/if}
  {/each}

  {#if app.toast}
    <div class="toast" role="status">{app.toast}</div>
  {/if}
</div>

<style>
  .shell { position: fixed; inset: 0; overflow: hidden; }
  .top { display: flex; align-items: center; justify-content: space-between; height: var(--header-row); padding: 0 16px; }
  .me, .status { width: 32px; height: 32px; display: grid; place-items: center; }
  .row { display: flex; gap: 8px; justify-content: center; flex-wrap: wrap; }
  .mark { width: 40px; height: 40px; display: grid; place-items: center; }
  .mark svg { width: 28px; height: 28px; fill: var(--text); }
  .mini-spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .tabbar {
    position: fixed;
    z-index: 10;
    left: 0;
    right: 0;
    bottom: 0;
    height: var(--nav-h);
    display: flex;
    background: var(--bg-blur);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-top: 1px solid var(--line);
    transition: opacity 220ms ease;
  }
  .tabbar.faded { opacity: 0.45; }
  .tabbar button { flex: 1; display: grid; place-items: center; position: relative; }
  .dot { position: absolute; top: 10px; left: calc(50% + 8px); width: 8px; height: 8px; border-radius: 50%; }
  .dot.busy { background: var(--accent); }
  .dot.alert { background: var(--danger); }
  .fab {
    position: fixed;
    z-index: 11;
    right: 16px;
    bottom: calc(var(--nav-h) + 16px);
    width: 56px;
    height: 56px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    background: var(--accent);
    box-shadow: 0 0 10px rgb(101 119 134 / 0.2), 0 1px 3px rgb(101 119 134 / 0.25);
    transition: opacity 220ms ease;
  }
  .fab:active { filter: brightness(0.9); }
  .fab.faded { opacity: 0.5; }
  .fresh {
    position: fixed;
    z-index: 12;
    top: calc(var(--header-row) + var(--tabs-row) + 12px);
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    border-radius: 9999px;
    color: #fff;
    background: var(--accent);
    font-weight: 700;
    box-shadow: 0 1px 8px rgb(0 0 0 / 0.25);
    transition: top 220ms ease;
  }
  .fresh.raised { top: 12px; }
  .toast {
    position: fixed;
    z-index: 60;
    left: 50%;
    bottom: calc(var(--nav-h) + 24px);
    transform: translateX(-50%);
    max-width: calc(100% - 32px);
    padding: 12px 16px;
    border-radius: 4px;
    color: #fff;
    background: var(--accent);
    box-shadow: 0 1px 8px rgb(0 0 0 / 0.2);
    animation: pop 180ms ease;
  }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, 8px); } }
</style>
