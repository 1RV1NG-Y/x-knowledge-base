<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { app, defaultLatest, filterCount, type LatestOptions } from './state.svelte';

  // Each change applies at once (the feed behind reloads), like the desktop filter panel.
  const options = $derived(app.latest);
  let author = $state(app.latest.author);

  function set<K extends keyof LatestOptions>(key: K, value: LatestOptions[K]) {
    if (options[key] === value) return;
    app.setLatest({ ...options, [key]: value });
  }

  function commitAuthor() {
    set('author', author.trim().replace(/^@/, ''));
  }

  // Closed with Back while typing: keep what was typed.
  onDestroy(commitAuthor);

  function reset() {
    author = '';
    app.setLatest({ ...defaultLatest });
  }

  function done() {
    commitAuthor();
    app.pop();
  }

  const sorts = [
    { id: 'newest', label: 'Newest first', hint: 'Most recent posts at the top' },
    { id: 'oldest', label: 'Oldest first', hint: 'Start from the beginning' },
  ] as const;
  const media = [
    { id: '', label: 'All' },
    { id: 'image', label: 'Photos' },
    { id: 'video', label: 'Videos' },
  ] as const;
  const statuses = [
    { id: '', label: 'Any' },
    { id: 'ok', label: 'Enriched' },
    { id: 'pending', label: 'Pending' },
    { id: 'failed', label: 'Failed' },
    { id: 'unavailable', label: 'Unavailable' },
  ];
  const changed = $derived(options.sort !== 'newest' || filterCount(options) > 0);
</script>

<div class="scrim" role="presentation" onclick={done}></div>
<div class="sheet" role="dialog" aria-modal="true" aria-label="Sort and filter">
  <div class="grip" aria-hidden="true"></div>

  <h2>Sort</h2>
  {#each sorts as sort (sort.id)}
    <button type="button" class="row" aria-pressed={options.sort === sort.id} onclick={() => set('sort', sort.id)}>
      <span><strong>{sort.label}</strong><small>{sort.hint}</small></span>
      {#if options.sort === sort.id}<span class="tick"><Icon name="check" size={20} /></span>{/if}
    </button>
  {/each}

  <h2>Media</h2>
  <div class="chips">
    {#each media as item (item.id)}
      <button type="button" class:on={options.media === item.id} onclick={() => set('media', item.id)}>{item.label}</button>
    {/each}
  </div>

  <h2>Posted</h2>
  <div class="dates">
    <label>
      <span>After</span>
      <input type="date" value={options.after} max={options.before || undefined} onchange={(e) => set('after', e.currentTarget.value)} />
    </label>
    <label>
      <span>Before</span>
      <input type="date" value={options.before} min={options.after || undefined} onchange={(e) => set('before', e.currentTarget.value)} />
    </label>
  </div>

  <h2>From account</h2>
  <form class="author" onsubmit={(e) => { e.preventDefault(); commitAuthor(); (document.activeElement as HTMLElement | null)?.blur(); }}>
    <span class="at">@</span>
    <input
      type="text"
      bind:value={author}
      onblur={commitAuthor}
      placeholder="username"
      autocapitalize="none"
      autocomplete="off"
      spellcheck="false"
      enterkeyhint="done"
    />
    {#if author}
      <button type="button" class="clear" aria-label="Clear account" onclick={() => { author = ''; commitAuthor(); }}><Icon name="close" size={16} /></button>
    {/if}
  </form>

  <h2>Status</h2>
  <div class="chips wrap">
    {#each statuses as item (item.id)}
      <button type="button" class:on={options.status === item.id} onclick={() => set('status', item.id)}>{item.label}</button>
    {/each}
  </div>

  <div class="actions">
    <button type="button" class="pill outline" disabled={!changed} onclick={reset}>Reset</button>
    <button type="button" class="pill primary" onclick={done}>Done</button>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; z-index: 30; background: var(--scrim); animation: fade 160ms ease; }
  .sheet {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 31;
    max-height: 88%;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 8px 0 16px;
    border-radius: 16px 16px 0 0;
    background: var(--sheet);
    box-shadow: 0 0 15px rgb(101 119 134 / 0.2);
    animation: rise 200ms ease;
  }
  .grip { width: 36px; height: 4px; border-radius: 2px; background: var(--line-strong); margin: 0 auto 4px; }
  h2 { font-size: 15px; font-weight: 800; margin: 16px 16px 8px; }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 16px;
    text-align: left;
  }
  .row:active { background: var(--hover); }
  .row span:first-child { display: grid; }
  .row small { color: var(--muted); font-size: 14px; }
  .tick { color: var(--accent); display: grid; }
  .chips { display: flex; gap: 8px; padding: 0 16px; overflow-x: auto; scrollbar-width: none; }
  .chips.wrap { flex-wrap: wrap; }
  .chips button {
    flex: none;
    padding: 6px 14px;
    border-radius: 9999px;
    border: 1px solid var(--line-strong);
    font-weight: 700;
    font-size: 14px;
  }
  .chips button.on { background: var(--text); color: var(--bg); border-color: var(--text); }
  .dates { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; padding: 0 16px; }
  label { display: grid; gap: 4px; }
  label span { color: var(--muted); font-size: 13px; }
  input {
    width: 100%;
    min-height: 44px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid var(--line-strong);
    background: var(--bg);
    color-scheme: light dark;
  }
  input:focus { outline: 2px solid var(--accent); border-color: transparent; }
  .author { position: relative; margin: 0 16px; }
  .author input { padding-left: 30px; padding-right: 40px; }
  .at { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); color: var(--muted); }
  .clear {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .actions { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 24px 16px 0; }
  .actions .pill { min-height: 44px; }
  @keyframes rise { from { transform: translateY(100%); } }
  @keyframes fade { from { opacity: 0; } }
</style>
