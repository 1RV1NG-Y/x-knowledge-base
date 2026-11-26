<script lang="ts">
  import Icon from '../../mobile/src/Icon.svelte';
  import { defaultLatest, desk, filterCount, type LatestOptions } from '../state.svelte';
  import Menu from './Menu.svelte';

  // Sort and filters for the Latest tab; each change applies at once.
  let { anchor, onclose }: { anchor: HTMLElement; onclose: () => void } = $props();

  const options = $derived(desk.latest);
  let author = $state(desk.latest.author);

  function set<K extends keyof LatestOptions>(key: K, value: LatestOptions[K]) {
    if (options[key] === value) return;
    desk.setLatest({ ...options, [key]: value });
  }
  const commitAuthor = () => set('author', author.trim().replace(/^@/, ''));

  const media = [
    { id: '', label: 'All' },
    { id: 'image', label: 'Photos' },
    { id: 'video', label: 'Videos' }
  ] as const;
  const statuses = [
    { id: '', label: 'Any' },
    { id: 'ok', label: 'Enriched' },
    { id: 'pending', label: 'Pending' },
    { id: 'failed', label: 'Failed' },
    { id: 'unavailable', label: 'Unavailable' }
  ];
  const changed = $derived(options.sort !== 'newest' || filterCount(options) > 0);
</script>

<Menu {anchor} {onclose} label="Sort and filter" align="start" width={380}>
  <div class="panel">
    <h3>Sort</h3>
    <div class="seg">
      <button type="button" class:on={options.sort === 'newest'} onclick={() => set('sort', 'newest')}>
        {#if options.sort === 'newest'}<Icon name="check" size={16} />{/if}Newest first
      </button>
      <button type="button" class:on={options.sort === 'oldest'} onclick={() => set('sort', 'oldest')}>
        {#if options.sort === 'oldest'}<Icon name="check" size={16} />{/if}Oldest first
      </button>
    </div>

    <h3>Media</h3>
    <div class="chips">
      {#each media as item (item.id)}
        <button type="button" class:on={options.media === item.id} onclick={() => set('media', item.id)}>{item.label}</button>
      {/each}
    </div>

    <h3>Posted</h3>
    <div class="dates">
      <label><span>After</span><input class="field" type="date" value={options.after} max={options.before || undefined} onchange={(e) => set('after', e.currentTarget.value)} /></label>
      <label><span>Before</span><input class="field" type="date" value={options.before} min={options.after || undefined} onchange={(e) => set('before', e.currentTarget.value)} /></label>
    </div>

    <h3>From account</h3>
    <form class="author" onsubmit={(e) => { e.preventDefault(); commitAuthor(); }}>
      <span class="at">@</span>
      <input class="field" type="text" bind:value={author} onblur={commitAuthor} placeholder="username" autocomplete="off" spellcheck="false" />
    </form>

    <h3>Status</h3>
    <div class="chips">
      {#each statuses as item (item.id)}
        <button type="button" class:on={options.status === item.id} onclick={() => set('status', item.id)}>{item.label}</button>
      {/each}
    </div>

    <div class="foot">
      <button type="button" class="pill outline small" disabled={!changed} onclick={() => { author = ''; desk.setLatest({ ...defaultLatest }); }}>Reset</button>
      <button type="button" class="pill primary small" onclick={() => { commitAuthor(); onclose(); }}>Done</button>
    </div>
  </div>
</Menu>

<style>
  .panel { padding: 4px 16px 12px; }
  h3 { font-size: 15px; font-weight: 800; margin: 14px 0 8px; }
  .seg { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .seg button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 36px;
    border-radius: 9999px;
    border: 1px solid var(--line-strong);
    font-weight: 700;
    font-size: 14px;
  }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .chips button { padding: 6px 14px; border-radius: 9999px; border: 1px solid var(--line-strong); font-weight: 700; font-size: 14px; }
  .seg button:hover:not(.on), .chips button:hover:not(.on) { background: var(--hover-strong); }
  .seg button.on, .chips button.on { background: var(--text); color: var(--bg); border-color: var(--text); }
  .dates { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  label { display: grid; gap: 4px; }
  label span { color: var(--muted); font-size: 13px; }
  .author { position: relative; }
  .author .field { padding-left: 28px; }
  .at { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--muted); }
  .foot { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
</style>
