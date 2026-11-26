<script lang="ts">
  import Icon from '../../mobile/src/Icon.svelte';
  import { ago } from '../../mobile/src/format';
  import { desk } from '../state.svelte';
  import ListTile from './ListTile.svelte';
  import SearchBox from './SearchBox.svelte';

  // The right column: search, the library at a glance, the phone and recent lists.
  const job = $derived(desk.job);
  const working = $derived(Boolean(job?.total && job.processed < job.total));
  const percent = $derived(job?.total ? Math.min(100, (job.processed / job.total) * 100) : 0);
  const phones = $derived(desk.sync?.status?.devices ?? []);
  const recent = $derived(desk.playlists.slice(0, 5));
  const onSearch = $derived(desk.section === 'search' && !desk.stack.length);
</script>

<aside class="side">
  {#if !onSearch}
    <div class="search"><SearchBox /></div>
  {/if}

  <section class="card">
    <h2>Your library</h2>
    <div class="stats">
      <div><strong>{desk.likeCount.toLocaleString()}</strong><span>liked posts</span></div>
      <div><strong>{desk.playlists.length.toLocaleString()}</strong><span>{desk.playlists.length === 1 ? 'list' : 'lists'}</span></div>
      {#if desk.profiles.profiles.length > 1}
        <div><strong>{desk.profiles.profiles.length}</strong><span>accounts</span></div>
      {/if}
    </div>
    {#if desk.importError}
      <p class="status bad"><strong>Import failed.</strong> {desk.importError}</p>
    {:else if working && job}
      <div class="status">
        <span><strong>{job.phase || 'Working'}</strong> · {job.processed.toLocaleString()} of {job.total.toLocaleString()}{job.failed ? ` · ${job.failed.toLocaleString()} failed` : ''}</span>
        <span class="bar"><span style:width="{percent}%"></span></span>
        {#if job.message}<span class="small">{job.message}</span>{/if}
      </div>
    {:else if desk.importSummary}
      {@const s = desk.importSummary}
      <p class="status"><strong>Import complete.</strong> {s.imported.toLocaleString()} new · {s.existing.toLocaleString()} already here · {s.pending.toLocaleString()} to fetch{s.failed ? ` · ${s.failed.toLocaleString()} failed` : ''}</p>
    {:else if job?.message}
      <p class="status"><strong>{job.phase || 'Ready'}.</strong> {job.message}</p>
    {/if}
  </section>

  <section class="card">
    <h2>Phone</h2>
    {#if phones.length}
      {#each phones as phone (phone.id)}
        <button type="button" class="row" onclick={() => (desk.phoneOpen = true)}>
          <span class="glyph" class:on={phone.connected}><Icon name="phone" size={20} /></span>
          <span class="info">
            <strong>{phone.name}</strong>
            <span>{phone.connected ? 'Connected · ' : ''}{phone.lastSync ? `synced ${ago(phone.lastSync)}` : 'not synced yet'}</span>
          </span>
        </button>
      {/each}
    {:else}
      <p class="small pad">Take your likes, lists and saved images along on Android. The phone keeps a copy and updates it over Wi-Fi while this app is open.</p>
      <div class="pad"><button type="button" class="pill primary small" onclick={() => (desk.phoneOpen = true)}>Pair a phone</button></div>
    {/if}
  </section>

  {#if recent.length}
    <section class="card">
      <h2>Lists</h2>
      {#each recent as list (list.id)}
        <button type="button" class="row" onclick={() => desk.open({ kind: 'list', playlist: list })}>
          <ListTile id={list.id} size={40} />
          <span class="info">
            <strong>{list.name}</strong>
            <span>{list.itemCount.toLocaleString()} {list.itemCount === 1 ? 'post' : 'posts'}</span>
          </span>
        </button>
      {/each}
      {#if desk.playlists.length > recent.length}
        <button type="button" class="all" onclick={() => desk.go('lists')}>Show all</button>
      {/if}
    </section>
  {/if}

  <p class="foot">Everything stays on this device.</p>
</aside>

<style>
  .side {
    width: var(--side-w);
    flex: none;
    margin-left: 30px;
    padding-bottom: 64px;
  }
  .search { position: sticky; top: 0; z-index: 3; padding: 6px 0 12px; background: var(--bg); }
  .card { margin-top: 16px; padding: 12px 0 4px; border: 1px solid var(--line); border-radius: 16px; overflow: hidden; }
  .search + .card { margin-top: 4px; }
  h2 { margin: 0 16px 12px; font-size: 20px; line-height: 24px; font-weight: 800; }
  .stats { display: flex; gap: 24px; padding: 0 16px 12px; }
  .stats div { display: grid; }
  .stats strong { font-size: 20px; line-height: 24px; }
  .stats span { color: var(--muted); font-size: 13px; }
  .status { margin: 0; padding: 12px 16px; border-top: 1px solid var(--line); display: grid; gap: 6px; font-size: 14px; color: var(--muted); }
  .status strong { color: var(--text); }
  .status.bad, .status.bad strong { color: var(--danger); }
  .bar { height: 4px; border-radius: 2px; background: var(--accent-soft); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transition: width 300ms linear; }
  .small { font-size: 13px; color: var(--muted); }
  .pad { margin: 0; padding: 0 16px 12px; }
  .row { width: 100%; display: flex; align-items: center; gap: 12px; padding: 12px 16px; text-align: left; transition: background-color 120ms ease; }
  .row:hover { background: var(--hover); }
  .glyph { width: 40px; height: 40px; border-radius: 50%; display: grid; place-items: center; background: var(--raised); color: var(--muted); flex: none; }
  .glyph.on { background: var(--accent-soft); color: var(--accent); }
  .info { display: grid; min-width: 0; line-height: 20px; }
  .info strong, .info span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .info span { color: var(--muted); font-size: 14px; }
  .all { width: 100%; padding: 16px; text-align: left; color: var(--accent); transition: background-color 120ms ease; }
  .all:hover { background: var(--hover); }
  .foot { margin: 16px; color: var(--muted); font-size: 13px; }
</style>
