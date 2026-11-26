<script lang="ts">
  import { inTauri, removeSyncDevice, scanCode, syncNow, useSyncCode } from './api';
  import { ago, bytes } from './format';
  import Icon from './Icon.svelte';
  import Page from './Page.svelte';
  import { app, message } from './state.svelte';

  let { active, onchrome }: { active: boolean; onchrome: (hidden: boolean) => void } = $props();

  let code = $state('');
  let pasting = $state(false);
  let busy = $state(false);
  let problem = $state<string>();
  let confirmUnpair = $state(false);

  const status = $derived(app.sync?.status);
  const pc = $derived(status?.devices[0]);
  const progress = $derived(status?.progress);
  const working = $derived(progress && ['checking', 'downloading', 'applying'].includes(progress.phase));
  const percent = $derived(progress && progress.bytesTotal > 0 ? Math.min(100, (progress.bytesDone / progress.bytesTotal) * 100) : 0);

  async function pair(text: string) {
    busy = true;
    problem = undefined;
    try {
      const name = await useSyncCode(text);
      code = '';
      pasting = false;
      app.showToast(`Paired with ${name}`);
      await app.refreshSync();
    } catch (cause) {
      problem = message(cause);
    } finally {
      busy = false;
    }
  }

  async function scan() {
    problem = undefined;
    try {
      const text = await scanCode();
      if (text) await pair(text);
    } catch (cause) {
      problem = message(cause);
    }
  }

  async function unpair() {
    if (!pc) return;
    try {
      await removeSyncDevice(pc.id);
      confirmUnpair = false;
      await app.refreshSync();
    } catch (cause) {
      problem = message(cause);
    }
  }
</script>

<Page {active} {onchrome}>
  {#snippet header()}
    <div class="top"><h1>Sync with PC</h1></div>
  {/snippet}

  {#if app.sync?.error}
    <p class="problem">{app.sync.error}</p>
  {/if}

  {#if pc}
    <section class="card">
      <div class="pc">
        <span class="icon" class:on={pc.connected}><Icon name="laptop" size={26} /></span>
        <div>
          <strong>{pc.name}</strong>
          <span class="muted">{pc.connected ? 'Connected' : 'Not connected'} · paired {ago(pc.pairedAt)}</span>
        </div>
      </div>
      {#if progress}
        <p class="phase" class:error={progress.phase === 'error'}>{progress.message}</p>
        {#if progress.phase === 'downloading' && progress.bytesTotal > 0}
          <div class="bar" role="progressbar" aria-valuenow={Math.round(percent)} aria-valuemin="0" aria-valuemax="100">
            <span style:width="{percent}%"></span>
          </div>
          <p class="muted small">
            {bytes(progress.bytesDone)} of {bytes(progress.bytesTotal)} · {progress.filesDone.toLocaleString()} of {progress.filesTotal.toLocaleString()} files.
            Keep the app open; the screen stays on until it’s done.
          </p>
        {/if}
        <dl>
          <div><dt>Last synced</dt><dd>{ago(progress.lastSync)}</dd></div>
          {#if progress.posts != null}<div><dt>Posts</dt><dd>{progress.posts.toLocaleString()}</dd></div>{/if}
        </dl>
      {/if}
      <div class="buttons">
        <button type="button" class="pill accent" disabled={Boolean(working)} onclick={() => void syncNow()}>
          <Icon name="sync" size={18} />{working ? 'Syncing…' : 'Sync now'}
        </button>
        {#if confirmUnpair}
          <button type="button" class="pill danger" onclick={() => void unpair()}>Unpair</button>
          <button type="button" class="pill outline" onclick={() => (confirmUnpair = false)}>Keep</button>
        {:else}
          <button type="button" class="pill outline" onclick={() => (confirmUnpair = true)}>Unpair…</button>
        {/if}
      </div>
    </section>
    <p class="hint">
      Your PC is the source: likes, lists and saved images come from it whenever X Knowledge Base is open
      there and both are on the same Wi-Fi. Nothing on the phone is sent back.
    </p>
  {:else}
    <div class="empty">
      <h2>Bring your library over</h2>
      <p>
        On the PC, open X Knowledge Base, click <b>Phone</b>, then <b>Pair a phone</b>. Scan the code it shows.
        Both devices need to be on the same Wi-Fi.
      </p>
      {#if inTauri}
        <button type="button" class="pill primary wide" disabled={busy} onclick={() => void scan()}>
          <Icon name="camera" size={20} />Scan the PC’s code
        </button>
      {/if}
      {#if pasting}
        <form onsubmit={(e) => { e.preventDefault(); void pair(code); }}>
          <textarea bind:value={code} rows="4" placeholder="KBX1…" autocapitalize="characters" spellcheck="false"></textarea>
          <button type="submit" class="pill accent wide" disabled={busy || !code.trim()}>{busy ? 'Pairing…' : 'Pair'}</button>
        </form>
      {:else}
        <button type="button" class="pill outline wide" onclick={() => (pasting = true)}>
          <Icon name="paste" size={18} />Paste a text code instead
        </button>
      {/if}
    </div>
  {/if}
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
</Page>

<style>
  .top { height: var(--header-row); display: flex; align-items: center; padding: 0 16px; }
  h1 { font-size: 20px; font-weight: 700; margin: 0; }
  .card { margin: 16px; padding: 16px; border: 1px solid var(--line); border-radius: 16px; }
  .pc { display: flex; gap: 12px; align-items: center; }
  .pc div { display: grid; }
  .icon { width: 48px; height: 48px; border-radius: 50%; display: grid; place-items: center; background: var(--skeleton); color: var(--muted); }
  .icon.on { background: var(--accent-soft); color: var(--accent); }
  .muted { color: var(--muted); }
  .small { font-size: 13px; line-height: 18px; margin: 8px 0 0; }
  .phase { margin: 16px 0 0; font-weight: 700; }
  .phase.error { color: var(--danger); font-weight: 400; }
  .bar { margin-top: 12px; height: 4px; border-radius: 2px; background: var(--accent-soft); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transition: width 250ms linear; }
  dl { margin: 16px 0 0; display: grid; gap: 4px; }
  dl div { display: flex; justify-content: space-between; }
  dt { color: var(--muted); }
  dd { margin: 0; font-weight: 700; }
  .buttons { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 16px; }
  .hint { margin: 0 16px; color: var(--muted); font-size: 14px; }
  .wide { width: 100%; min-height: 44px; margin-top: 12px; }
  textarea {
    width: 100%;
    margin-top: 16px;
    padding: 12px;
    border-radius: 8px;
    border: 1px solid var(--line-strong);
    background: var(--bg);
    font-family: ui-monospace, monospace;
    font-size: 13px;
    resize: none;
  }
  textarea:focus { outline: 2px solid var(--accent); border-color: transparent; }
  .problem { margin: 16px; padding: 12px; border-radius: 12px; color: var(--danger); background: rgb(244 33 46 / 0.1); }
</style>
