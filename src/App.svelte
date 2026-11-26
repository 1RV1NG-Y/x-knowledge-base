<script lang="ts">
  import { onMount, tick } from 'svelte';
  import ListDialog from './components/ListDialog.svelte';
  import Nav from './components/Nav.svelte';
  import PhoneDialog from './components/PhoneDialog.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import TitleBar from './components/TitleBar.svelte';
  import Viewer from './components/Viewer.svelte';
  import { onEvent } from './api';
  import { desk, type View } from './state.svelte';
  import HomeView from './views/HomeView.svelte';
  import ListsView from './views/ListsView.svelte';
  import ListView from './views/ListView.svelte';
  import PostView from './views/PostView.svelte';
  import ProfileView from './views/ProfileView.svelte';
  import SearchView from './views/SearchView.svelte';

  let scroller = $state<HTMLElement>();
  let home = $state<ReturnType<typeof HomeView>>();
  // Sections stay mounted once visited, so going back to one finds it as it was.
  let visited = $state<Record<string, boolean>>({ home: true });
  $effect(() => {
    if (!visited[desk.section]) visited = { ...visited, [desk.section]: true };
  });

  // Each page keeps its own scroll position in the shared middle column.
  const viewKey = (v: View) => (v.kind === 'post' ? `post:${v.tweet.id}` : v.kind === 'profile' ? `profile:${v.author.username}` : `list:${v.playlist.id}`);
  const key = $derived(
    desk.top ? `${desk.stack.length}:${viewKey(desk.top)}` : desk.section === 'home' ? `home:${desk.homeTab}` : desk.section
  );
  const positions = new Map<string, number>();
  let shownKey = '';

  $effect.pre(() => {
    const next = key;
    if (scroller && shownKey && next !== shownKey) positions.set(shownKey, scroller.scrollTop);
  });
  $effect(() => {
    const next = key;
    if (next === shownKey) return;
    // Pages above the one shown are gone now; forget where they were.
    const depth = desk.stack.length;
    for (const k of positions.keys()) {
      const level = Number(k.split(':')[0]);
      if (Number.isFinite(level) && level > depth) positions.delete(k);
    }
    shownKey = next;
    const top = positions.get(next) ?? 0;
    void tick().then(() => scroller?.scrollTo({ top }));
  });

  $effect(() => {
    desk.scroller = scroller;
  });

  onMount(() => {
    void Promise.all([desk.refreshProfiles(), desk.refreshPlaylists(), desk.refreshJob(), desk.refreshSync()]);
    const jobTimer = setInterval(() => void desk.refreshJob(), 1500);
    const syncTimer = setInterval(() => void desk.refreshSync(), 30000);
    const stopSync = onEvent('sync-status', () => void desk.refreshSync());
    // Agents edit lists straight in the database: pick up their changes on focus.
    const focus = () => void desk.refreshPlaylists();
    const typing = (target: EventTarget | null) =>
      target instanceof HTMLElement && Boolean(target.closest('input, textarea, select, [contenteditable]'));
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !typing(event.target)) {
        if (desk.back()) event.preventDefault();
      } else if (event.key === 'ArrowLeft' && event.altKey) {
        if (desk.back()) event.preventDefault();
      } else if (event.key === '/' && !typing(event.target)) {
        event.preventDefault();
        document.querySelector<HTMLInputElement>('input[data-search]')?.focus() ?? desk.go('search');
      }
    };
    // The mouse's back button.
    const mouse = (event: MouseEvent) => {
      if (event.button === 3 && desk.back()) event.preventDefault();
    };
    window.addEventListener('focus', focus);
    window.addEventListener('keydown', key);
    window.addEventListener('mouseup', mouse);
    return () => {
      clearInterval(jobTimer);
      clearInterval(syncTimer);
      void stopSync.then((f) => f());
      window.removeEventListener('focus', focus);
      window.removeEventListener('keydown', key);
      window.removeEventListener('mouseup', mouse);
    };
  });
</script>

<div class="app">
  <TitleBar />
  <div class="scroller" bind:this={scroller}>
    <div class="frame">
      <Nav onhome={() => (desk.homeTab === 'random' ? home?.shuffle() : desk.scrollTop())} />
      <main class="center">
        <div class="page" class:hidden={desk.section !== 'home' || desk.stack.length > 0}><HomeView bind:this={home} /></div>
        {#if visited.search}
          <div class="page" class:hidden={desk.section !== 'search' || desk.stack.length > 0}>
            <SearchView active={desk.section === 'search' && desk.stack.length === 0} />
          </div>
        {/if}
        {#if visited.lists}
          <div class="page" class:hidden={desk.section !== 'lists' || desk.stack.length > 0}><ListsView /></div>
        {/if}
        {#each desk.stack as view, i (`${i}:${viewKey(view)}`)}
          <div class="page" class:hidden={i !== desk.stack.length - 1}>
            {#if view.kind === 'post'}<PostView tweet={view.tweet} />
            {:else if view.kind === 'profile'}<ProfileView author={view.author} />
            {:else}<ListView playlist={view.playlist} />{/if}
          </div>
        {/each}
      </main>
      <div class="side-wrap"><Sidebar /></div>
    </div>
  </div>
</div>

{#if desk.viewer}<Viewer />{/if}
{#if desk.phoneOpen}<PhoneDialog />{/if}
{#if desk.listDialog}<ListDialog />{/if}
{#if desk.toast}<div class="toast" role="status">{desk.toast}</div>{/if}

<style>
  .app { height: 100%; display: flex; flex-direction: column; }
  .scroller { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; scrollbar-gutter: stable; }
  .frame { display: flex; justify-content: center; min-height: 100%; }
  .center {
    width: var(--center-w);
    flex: none;
    min-height: 100%;
    border-left: 1px solid var(--line);
    border-right: 1px solid var(--line);
  }
  .side-wrap { display: flex; }
  .side-wrap :global(.side) { position: sticky; top: 0; align-self: flex-start; }
  .hidden { display: none; }
  .toast {
    position: fixed;
    z-index: 70;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    max-width: calc(100% - 32px);
    padding: 12px 16px;
    border-radius: 4px;
    color: #fff;
    background: var(--accent);
    box-shadow: 0 1px 8px rgb(0 0 0 / 0.25);
    animation: toast 180ms ease;
  }
  @keyframes toast { from { opacity: 0; transform: translate(-50%, 8px); } }
  @media (max-width: 1079px) {
    .side-wrap { display: none; }
  }
  /* Scrollbar that fits the theme. */
  .scroller::-webkit-scrollbar { width: 10px; }
  .scroller::-webkit-scrollbar-thumb { background: var(--line); border-radius: 5px; border: 2px solid var(--bg); }
  .scroller::-webkit-scrollbar-thumb:hover { background: var(--line-strong); }
</style>
