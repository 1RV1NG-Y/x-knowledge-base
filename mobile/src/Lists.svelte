<script lang="ts">
  import Icon from './Icon.svelte';
  import Page from './Page.svelte';
  import { app } from './state.svelte';

  let { active, onchrome }: { active: boolean; onchrome: (hidden: boolean) => void } = $props();

  const hue = (id: number) => (id * 67) % 360;
</script>

<Page {active} {onchrome}>
  {#snippet header()}
    <div class="top"><h1>Lists</h1></div>
  {/snippet}
  {#each app.playlists as list (list.id)}
    <button type="button" class="row" onclick={() => app.push({ kind: 'playlist', playlist: list })}>
      <span class="tile" style:background="linear-gradient(135deg, hsl({hue(list.id)} 70% 55%), hsl({hue(list.id) + 40} 70% 40%))">
        <Icon name="lists" size={22} />
      </span>
      <span class="info">
        <strong>{list.name}</strong>
        {#if list.description}<span class="desc">{list.description}</span>{/if}
        <span class="muted">{list.itemCount.toLocaleString()} {list.itemCount === 1 ? 'post' : 'posts'}</span>
      </span>
    </button>
  {:else}
    <div class="empty">
      <h2>No lists yet</h2>
      <p>Lists you make on the PC show up here after the next sync.</p>
    </div>
  {/each}
</Page>

<style>
  .top { height: var(--header-row); display: flex; align-items: center; padding: 0 16px; }
  h1 { font-size: 20px; font-weight: 700; margin: 0; }
  .row { width: 100%; display: flex; gap: 12px; padding: 12px 16px; text-align: left; align-items: center; }
  .row:active { background: var(--hover); }
  .tile { width: 48px; height: 48px; border-radius: 12px; display: grid; place-items: center; color: #fff; flex: none; }
  .info { display: grid; min-width: 0; }
  .info strong, .desc { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .desc { font-size: 14px; }
  .muted { color: var(--muted); font-size: 14px; }
</style>
