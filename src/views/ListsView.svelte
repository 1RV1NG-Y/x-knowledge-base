<script lang="ts">
  import Icon from '../../mobile/src/Icon.svelte';
  import { ago } from '../../mobile/src/format';
  import ColumnHeader from '../components/ColumnHeader.svelte';
  import ListTile from '../components/ListTile.svelte';
  import { desk } from '../state.svelte';
</script>

<ColumnHeader title="Lists" subtitle="{desk.playlists.length} {desk.playlists.length === 1 ? 'list' : 'lists'}">
  {#snippet actions()}
    <button type="button" class="round" aria-label="New list" title="New list" onclick={() => (desk.listDialog = {})}><Icon name="plus" size={20} /></button>
  {/snippet}
</ColumnHeader>

{#if desk.playlistError}<p class="problem">{desk.playlistError}</p>{/if}

{#each desk.playlists as list (list.id)}
  <button type="button" class="row" onclick={() => desk.open({ kind: 'list', playlist: list })}>
    <ListTile id={list.id} />
    <span class="info">
      <strong>{list.name}</strong>
      {#if list.description}<span class="desc">{list.description}</span>{/if}
      <span class="muted">{list.itemCount.toLocaleString()} {list.itemCount === 1 ? 'post' : 'posts'} · updated {ago(list.updatedAt)}</span>
    </span>
  </button>
{:else}
  <div class="empty">
    <h2>No lists yet</h2>
    <p>Group posts by topic. Make a list here, or save any post to one with its bookmark button. Agents can fill lists too.</p>
    <button type="button" class="pill accent" onclick={() => (desk.listDialog = {})}>New list</button>
  </div>
{/each}

<style>
  .row { width: 100%; display: flex; gap: 12px; padding: 12px 16px; text-align: left; align-items: center; transition: background-color 120ms ease; }
  .row:hover { background: var(--hover); }
  .info { display: grid; min-width: 0; }
  .info strong, .desc { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .desc { font-size: 14px; }
  .muted { color: var(--muted); font-size: 14px; }
  .problem { margin: 16px; padding: 12px; border-radius: 12px; color: var(--danger); background: var(--danger-soft); }
</style>
