<script lang="ts">
  import type { MediaType } from '../../src/lib/types';
  import Avatar from './Avatar.svelte';
  import FeedList from './FeedList.svelte';
  import Icon from './Icon.svelte';
  import Page from './Page.svelte';
  import { app, Feed } from './state.svelte';

  let { active, onchrome }: { active: boolean; onchrome: (hidden: boolean) => void } = $props();

  let input = $state('');
  let media = $state<MediaType | ''>('');
  let scroller = $state<HTMLElement>();
  let field = $state<HTMLInputElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;

  // "from:handle" narrows to one author; the rest is a full-text search.
  const parsed = $derived.by(() => {
    let author: string | undefined;
    const words = input.trim().split(/\s+/).filter((word) => {
      const match = /^from:@?(\w+)$/i.exec(word);
      if (match) author = match[1];
      return !match && word;
    });
    return { search: words.join(' ') || undefined, author };
  });
  const asking = $derived(Boolean(parsed.search || parsed.author || media));
  const feed = new Feed(() => ({ ...parsed, mediaType: media || undefined, sortOrder: 'newest' }));

  function run() {
    scroller?.scrollTo({ top: 0 });
    if (asking) void feed.reset();
  }

  function typed() {
    clearTimeout(timer);
    timer = setTimeout(run, 300);
  }

  function chip(value: MediaType | '') {
    media = media === value ? '' : value;
    run();
  }

  $effect(() => {
    if (active && !asking) field?.focus();
  });
</script>

<Page {active} {onchrome} onend={asking ? () => void feed.more() : undefined} bind:scroller>
  {#snippet header()}
    <div class="top">
      <button type="button" class="me" aria-label="Account" onclick={() => app.push({ kind: 'drawer' })}>
        <Avatar path={app.activeProfile?.avatarLocalPath} url={app.activeProfile?.avatarUrl} name={app.activeProfile?.displayName ?? '?'} size={32} />
      </button>
      <label class="field">
        <Icon name="search" size={16} />
        <input
          bind:this={field}
          bind:value={input}
          oninput={typed}
          onkeydown={(e) => e.key === 'Enter' && (e.currentTarget.blur(), run())}
          type="search"
          placeholder="Search your likes"
          enterkeyhint="search"
          autocomplete="off"
        />
      </label>
    </div>
    <div class="chips">
      <button type="button" class:on={media === 'image'} onclick={() => chip('image')}><Icon name="photo" size={16} />Photos</button>
      <button type="button" class:on={media === 'video'} onclick={() => chip('video')}><Icon name="play" size={14} />Videos</button>
    </div>
  {/snippet}
  {#if asking}
    {#if feed.started && !feed.loading && feed.total > 0}
      <p class="count">{feed.total.toLocaleString()} {feed.total === 1 ? 'post' : 'posts'}</p>
    {/if}
    <FeedList {feed}>
      {#snippet empty()}
        <div class="empty">
          <h2>No results{parsed.search ? ` for “${parsed.search}”` : ''}</h2>
          <p>Try other words, or check the spelling.</p>
        </div>
      {/snippet}
    </FeedList>
  {:else}
    <div class="empty">
      <h2>Search your likes</h2>
      <p>Words from the post, the author’s name, or <b>from:username</b> for one author’s posts.</p>
    </div>
  {/if}
</Page>

<style>
  .top { display: flex; align-items: center; gap: 16px; height: var(--header-row); padding: 0 16px; }
  .me { flex: none; }
  .field {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 40px;
    padding: 0 16px;
    border-radius: 9999px;
    background: var(--skeleton);
    color: var(--muted);
    border: 1px solid transparent;
  }
  .field:focus-within { border-color: var(--accent); background: var(--bg); color: var(--accent); }
  input { flex: 1; min-width: 0; border: 0; outline: 0; background: none; color: var(--text); font-size: 15px; }
  input::placeholder { color: var(--muted); }
  input::-webkit-search-cancel-button { display: none; }
  .chips { display: flex; gap: 8px; padding: 0 16px 12px; }
  .chips button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    border-radius: 9999px;
    border: 1px solid var(--line-strong);
    font-weight: 700;
    font-size: 14px;
  }
  .chips button.on { background: var(--text); color: var(--bg); border-color: var(--text); }
  .count { margin: 0; padding: 12px 16px; color: var(--muted); border-bottom: 1px solid var(--line); }
</style>
