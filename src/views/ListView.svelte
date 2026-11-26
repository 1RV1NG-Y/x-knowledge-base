<script lang="ts">
  import { onMount } from 'svelte';
  import type { Playlist, SortOrder } from '../lib/types';
  import Icon from '../../mobile/src/Icon.svelte';
  import Tabs from '../../mobile/src/Tabs.svelte';
  import { deletePlaylist } from '../api';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import FeedList from '../components/FeedList.svelte';
  import ListTile from '../components/ListTile.svelte';
  import Menu from '../components/Menu.svelte';
  import { desk, message, newSeed, useFeed } from '../state.svelte';

  let { playlist }: { playlist: Playlist } = $props();

  let order = $state<SortOrder>('newest');
  let seed = newSeed();
  const feed = useFeed(() => ({ playlistId: playlist.id, sortOrder: order, seed }));
  let moreButton = $state<HTMLButtonElement>();
  let menu = $state(false);
  let confirming = $state(false);
  let problem = $state<string>();

  function change(id: string) {
    if (id === order && id !== 'random') return desk.scrollTop();
    if (id === 'random') seed = newSeed();
    order = id as SortOrder;
    desk.scrollTop(false);
    void feed.reset();
  }

  async function remove() {
    try {
      await deletePlaylist(playlist.id);
      confirming = false;
      desk.back();
      await desk.refreshPlaylists();
      desk.showToast(`Deleted “${playlist.name}”`);
    } catch (cause) {
      problem = message(cause);
    }
  }

  // Posts added or removed (here, or by an agent while the window was away).
  let updated = '';
  $effect(() => {
    const stamp = playlist.updatedAt;
    if (stamp === updated) return;
    const first = !updated;
    updated = stamp;
    if (!first) void feed.refresh();
  });

  onMount(() => void feed.reset());
</script>

<ColumnHeader back title={playlist.name} subtitle="{playlist.itemCount.toLocaleString()} {playlist.itemCount === 1 ? 'post' : 'posts'}">
  {#snippet actions()}
    <button type="button" class="round" aria-label="List actions" bind:this={moreButton} onclick={() => (menu = !menu)}><Icon name="more" size={20} /></button>
  {/snippet}
  <Tabs tabs={[{ id: 'newest', label: 'Newest' }, { id: 'oldest', label: 'Oldest' }, { id: 'random', label: 'Shuffled' }]} value={order} onchange={change} />
</ColumnHeader>

<section class="intro">
  <ListTile id={playlist.id} size={64} />
  <div>
    <h2>{playlist.name}</h2>
    {#if playlist.description}<p>{playlist.description}</p>{/if}
  </div>
  <button type="button" class="pill outline small" onclick={() => (desk.listDialog = { playlist })}>Edit list</button>
</section>
{#if problem}<p class="problem">{problem}</p>{/if}

<FeedList {feed}>
  {#snippet empty()}
    <div class="empty">
      <h2>Nothing here yet</h2>
      <p>Save posts to this list with the bookmark button under any post.</p>
    </div>
  {/snippet}
</FeedList>

{#if menu && moreButton}
  <Menu anchor={moreButton} onclose={() => (menu = false)} label="List actions">
    <button type="button" class="item" onclick={() => { menu = false; desk.listDialog = { playlist }; }}><Icon name="edit" size={18} />Edit list</button>
    <button type="button" class="item danger" onclick={() => { menu = false; confirming = true; }}><Icon name="trash" size={18} />Delete list</button>
  </Menu>
{/if}

{#if confirming}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-scrim" onclick={(e) => { if (e.target === e.currentTarget) confirming = false; }}>
    <div class="dialog confirm" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
      <h2 id="confirm-title">Delete list?</h2>
      <p>“{playlist.name}” goes away. The posts stay in your likes.</p>
      <button type="button" class="pill danger wide" onclick={() => void remove()}>Delete</button>
      <button type="button" class="pill outline wide" onclick={() => (confirming = false)}>Cancel</button>
    </div>
  </div>
{/if}

<style>
  .intro { display: flex; align-items: center; gap: 16px; padding: 16px; border-bottom: 1px solid var(--line); }
  .intro div { flex: 1; min-width: 0; }
  h2 { margin: 0; font-size: 20px; line-height: 24px; font-weight: 800; }
  .intro p { margin: 4px 0 0; color: var(--muted); white-space: pre-wrap; }
  .problem { margin: 16px; padding: 12px; border-radius: 12px; color: var(--danger); background: var(--danger-soft); }
  .confirm { width: 320px; padding: 32px; display: grid; gap: 12px; margin-top: 20vh; }
  .confirm p { margin: 0 0 12px; color: var(--muted); }
  .wide { width: 100%; min-height: 44px; }
</style>
