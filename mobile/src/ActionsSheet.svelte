<script lang="ts">
  import type { Tweet } from '../../src/lib/types';
  import { openUrl } from './api';
  import Icon from './Icon.svelte';
  import { app } from './state.svelte';

  let { tweet }: { tweet: Tweet } = $props();
  const lists = $derived(app.playlists.filter((p) => tweet.playlistIds.includes(p.id)));

  async function copy() {
    app.pop();
    try {
      await navigator.clipboard.writeText(tweet.canonicalUrl);
      app.showToast('Copied to clipboard');
    } catch {
      app.showToast('Couldn’t copy the link');
    }
  }
</script>

<div class="scrim" role="presentation" onclick={() => app.pop()}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Post actions">
  <div class="grip" aria-hidden="true"></div>
  <button type="button" onclick={() => { app.pop(); void openUrl(tweet.canonicalUrl); }}><Icon name="external" size={20} />Open on X</button>
  <button type="button" onclick={() => void copy()}><Icon name="link" size={20} />Copy link to post</button>
  {#if lists.length}
    <p class="label">In your lists</p>
    {#each lists as list (list.id)}
      <button type="button" onclick={() => { app.pop(); app.push({ kind: 'playlist', playlist: list }); }}>
        <Icon name="bookmarkFill" size={20} />{list.name}
      </button>
    {/each}
  {:else}
    <p class="label">Not in any list. Lists are made on the PC.</p>
  {/if}
  <button type="button" class="pill outline cancel" onclick={() => app.pop()}>Cancel</button>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 30; background: var(--scrim); animation: fade 160ms ease; }
  .sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 31;
    max-height: 80%;
    overflow-y: auto;
    padding: 8px 0 16px;
    border-radius: 16px 16px 0 0;
    background: var(--sheet);
    box-shadow: 0 0 15px rgb(101 119 134 / 0.2);
    animation: rise 200ms ease;
  }
  .grip { width: 36px; height: 4px; border-radius: 2px; background: var(--line-strong); margin: 0 auto 8px; }
  .sheet > button:not(.cancel) {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    font-weight: 700;
    text-align: left;
  }
  .sheet > button:not(.cancel):active { background: var(--hover); }
  .label { color: var(--muted); padding: 12px 16px 4px; margin: 0; font-size: 14px; }
  .cancel { display: flex; width: calc(100% - 32px); margin: 16px 16px 0; min-height: 44px; }
  @keyframes rise { from { transform: translateY(100%); } }
  @keyframes fade { from { opacity: 0; } }
</style>
