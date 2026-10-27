<script lang="ts">
  import { tick } from 'svelte';
  import type { Playlist } from '../types';

  interface Props {
    playlists: Playlist[];
    selectedIds: number[];
    ontoggle: (playlistId: number, included: boolean) => Promise<void>;
    oncreate: (name: string) => Promise<boolean>;
  }

  let { playlists, selectedIds, ontoggle, oncreate }: Props = $props();
  let open = $state(false);
  let busyId = $state<number>();
  let saving = $state(false);
  let draft = $state('');
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();

  async function showMenu(): Promise<void> {
    open = true;
    draft = '';
    await tick();
    (menu?.querySelector<HTMLElement>('[role="menuitemcheckbox"]') ?? menu?.querySelector<HTMLInputElement>('input'))?.focus();
  }

  function closeMenu(restoreFocus = false): void {
    open = false;
    if (restoreFocus) trigger?.focus();
  }

  async function toggle(playlist: Playlist): Promise<void> {
    if (busyId !== undefined) return;
    busyId = playlist.id;
    try {
      await ontoggle(playlist.id, !selectedIds.includes(playlist.id));
    } finally {
      busyId = undefined;
    }
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!draft.trim() || saving) return;
    saving = true;
    try {
      if (await oncreate(draft)) draft = '';
    } finally {
      saving = false;
    }
  }

  function handleWindowClick(event: MouseEvent): void {
    if (open && !menu?.contains(event.target as Node) && !trigger?.contains(event.target as Node)) closeMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      closeMenu(true);
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="playlist-menu-wrap">
  <button
    bind:this={trigger}
    type="button"
    class="text-button"
    class:saved={selectedIds.length > 0}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={() => (open ? closeMenu() : void showMenu())}
  >
    {selectedIds.length ? `In ${selectedIds.length} ${selectedIds.length === 1 ? 'playlist' : 'playlists'}` : 'Add to playlist'}
  </button>
  {#if open}
    <div class="playlist-menu" role="menu" aria-label="Playlists" bind:this={menu}>
      {#each playlists as playlist (playlist.id)}
        <button
          type="button"
          role="menuitemcheckbox"
          aria-checked={selectedIds.includes(playlist.id)}
          disabled={busyId !== undefined}
          onclick={() => void toggle(playlist)}
        >
          <span class="playlist-checkbox" aria-hidden="true">
            {#if selectedIds.includes(playlist.id)}<svg viewBox="0 0 24 24"><path d="m5 12.5 4.5 4.5L19 7.5" /></svg>{/if}
          </span>
          <span>{playlist.name}</span>
        </button>
      {/each}
      <form onsubmit={(event) => void submit(event)}>
        <input bind:value={draft} placeholder="New playlist…" aria-label="Create playlist and add this post" maxlength="80" disabled={saving} />
      </form>
    </div>
  {/if}
</div>
