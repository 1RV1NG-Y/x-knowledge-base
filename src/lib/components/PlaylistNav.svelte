<script lang="ts">
  import { tick } from 'svelte';
  import type { Playlist } from '../types';

  interface Props {
    playlists: Playlist[];
    activePlaylistId?: number;
    likeCount: number;
    error?: string;
    onselect: (playlistId?: number) => void;
    oncreate: (name: string) => Promise<boolean>;
  }

  let { playlists, activePlaylistId, likeCount, error, onselect, oncreate }: Props = $props();
  let creating = $state(false);
  let saving = $state(false);
  let draft = $state('');
  let input = $state<HTMLInputElement>();

  async function startCreating(): Promise<void> {
    creating = true;
    draft = '';
    await tick();
    input?.focus();
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!draft.trim() || saving) return;
    saving = true;
    try {
      if (await oncreate(draft)) creating = false;
    } finally {
      saving = false;
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      creating = false;
    }
  }
</script>

<section class="playlist-nav" aria-labelledby="playlist-nav-title">
  <div class="section-heading">
    <h2 id="playlist-nav-title">Playlists</h2>
    <button class="add-playlist" type="button" aria-label="New playlist" title="New playlist" onclick={() => void startCreating()}>
      <svg aria-hidden="true" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
    </button>
  </div>
  <ul>
    <li>
      <button type="button" class:active={activePlaylistId === undefined} aria-current={activePlaylistId === undefined ? 'page' : undefined} onclick={() => onselect(undefined)}>
        <span>All likes</span><small>{likeCount.toLocaleString()}</small>
      </button>
    </li>
    {#each playlists as playlist (playlist.id)}
      <li>
        <button
          type="button"
          class:active={activePlaylistId === playlist.id}
          aria-current={activePlaylistId === playlist.id ? 'page' : undefined}
          title={playlist.description || playlist.name}
          onclick={() => onselect(playlist.id)}
        >
          <span>{playlist.name}</span><small>{playlist.itemCount.toLocaleString()}</small>
        </button>
      </li>
    {/each}
  </ul>
  {#if creating}
    <form class="playlist-create" onsubmit={(event) => void submit(event)}>
      <input bind:this={input} bind:value={draft} placeholder="Playlist name" aria-label="New playlist name" maxlength="80" disabled={saving} onkeydown={handleKeydown} />
      <button class="secondary-button" type="submit" disabled={!draft.trim() || saving}>Add</button>
    </form>
  {:else if !playlists.length}
    <p class="playlist-hint">Group posts by topic. Create one here or from any post.</p>
  {/if}
  {#if error}<p class="playlist-error" role="alert">{error}</p>{/if}
</section>
