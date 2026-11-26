<script lang="ts">
  import type { Tweet } from '../lib/types';
  import Icon from '../../mobile/src/Icon.svelte';
  import { toggleList } from '../actions';
  import { desk } from '../state.svelte';
  import Menu from './Menu.svelte';

  // The bookmark button's menu: which lists this post is in.
  let { tweet, anchor, onclose }: { tweet: Tweet; anchor: HTMLElement; onclose: () => void } = $props();
  let busy = $state<number>();

  async function toggle(id: number) {
    if (busy != null) return;
    busy = id;
    await toggleList(tweet, id);
    busy = undefined;
  }
</script>

<Menu {anchor} {onclose} label="Lists" align="start" width={300}>
  <p class="label">Save to a list</p>
  {#each desk.playlists as list (list.id)}
    {@const on = tweet.playlistIds.includes(list.id)}
    <button type="button" class="item" role="menuitemcheckbox" aria-checked={on} disabled={busy != null} onclick={() => void toggle(list.id)}>
      <span class="box" class:on>{#if on}<Icon name="check" size={14} />{/if}</span>
      <span class="name">{list.name}</span>
      <span class="count">{list.itemCount.toLocaleString()}</span>
    </button>
  {:else}
    <p class="label">No lists yet.</p>
  {/each}
  <button type="button" class="item new" onclick={() => { onclose(); desk.listDialog = { addTweet: tweet.id }; }}>
    <Icon name="plus" size={18} />New list
  </button>
</Menu>

<style>
  .box {
    width: 20px;
    height: 20px;
    border-radius: 4px;
    border: 2px solid var(--line-strong);
    display: grid;
    place-items: center;
    flex: none;
    color: #fff;
  }
  .box.on { background: var(--accent); border-color: var(--accent); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 400; }
  .count { color: var(--muted); font-weight: 400; font-size: 13px; }
  .new { color: var(--accent); border-top: 1px solid var(--line); }
</style>
