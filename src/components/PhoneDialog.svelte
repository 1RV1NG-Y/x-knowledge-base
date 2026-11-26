<script lang="ts">
  import { onMount } from 'svelte';
  import type { SyncInvite } from '../lib/types';
  import Icon from '../../mobile/src/Icon.svelte';
  import { ago } from '../../mobile/src/format';
  import { createSyncInvite, onEvent, removeSyncDevice } from '../api';
  import { desk, message } from '../state.svelte';

  // Pair the Android app and see the paired phones.
  let invite = $state<SyncInvite>();
  let error = $state<string>();
  let copied = $state(false);
  let confirmRemove = $state<string>();

  const info = $derived(desk.sync);
  const devices = $derived(info?.status?.devices ?? []);
  let paired = 0;

  // The phone just paired: the code has done its job.
  $effect(() => {
    const count = devices.length;
    if (invite && count > paired) invite = undefined;
    paired = count;
  });

  async function showCode() {
    error = undefined;
    copied = false;
    try {
      invite = await createSyncInvite();
    } catch (cause) {
      error = message(cause);
    }
  }

  async function copyCode() {
    if (!invite) return;
    await navigator.clipboard.writeText(invite.ticket);
    copied = true;
  }

  async function remove(id: string) {
    error = undefined;
    try {
      await removeSyncDevice(id);
      confirmRemove = undefined;
      await desk.refreshSync();
    } catch (cause) {
      error = message(cause);
    }
  }

  const close = () => (desk.phoneOpen = false);

  onMount(() => {
    void desk.refreshSync();
    const stop = onEvent('sync-status', () => void desk.refreshSync());
    const timer = setInterval(() => void desk.refreshSync(), 15000);
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
    window.addEventListener('keydown', key, true);
    return () => {
      clearInterval(timer);
      void stop.then((f) => f());
      window.removeEventListener('keydown', key, true);
    };
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="dialog-scrim" onclick={(e) => { if (e.target === e.currentTarget) close(); }}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="phone-title">
    <header>
      <button type="button" class="round" aria-label="Close" onclick={close}><Icon name="close" size={20} /></button>
      <h2 id="phone-title">Phone</h2>
    </header>
    <div class="body">
      <p class="lede">
        The Android app keeps a read-only copy of your likes, lists and saved images, and updates it
        whenever this app is open on the same Wi-Fi.
      </p>

      {#if info?.error}<p class="problem" role="alert">{info.error}</p>{/if}

      {#each devices as device (device.id)}
        <div class="device">
          <span class="glyph" class:on={device.connected}><Icon name="phone" size={22} /></span>
          <div class="info">
            <strong>{device.name}</strong>
            <span>{device.connected ? 'Connected · ' : ''}{device.lastSync ? `synced ${ago(device.lastSync)}` : 'not synced yet'}</span>
          </div>
          {#if confirmRemove === device.id}
            <button type="button" class="pill danger small" onclick={() => void remove(device.id)}>Unpair</button>
            <button type="button" class="pill outline small" onclick={() => (confirmRemove = undefined)}>Keep</button>
          {:else}
            <button type="button" class="pill outline small" onclick={() => (confirmRemove = device.id)}>Unpair…</button>
          {/if}
        </div>
      {/each}

      {#if invite}
        <div class="invite">
          <div class="qr" aria-label="Pairing QR code">{@html invite.qrSvg}</div>
          <div>
            <strong>On your phone, open X Knowledge Base, go to Sync and tap “Scan the PC’s code”.</strong>
            <p>The code works once and expires in 10 minutes. No camera? Paste this text code instead:</p>
            <code>{invite.ticket}</code>
            <button type="button" class="pill outline small" onclick={() => void copyCode()}>{copied ? 'Copied' : 'Copy text code'}</button>
          </div>
        </div>
      {:else if info?.status}
        <button type="button" class="pill primary pair" onclick={() => void showCode()}>{devices.length ? 'Pair another phone' : 'Pair a phone'}</button>
      {/if}

      {#if error}<p class="problem" role="alert">{error}</p>{/if}
      {#if info?.status}
        <p class="foot">Uses UDP port {info.status.port ?? '—'} on your local network. If the phone can’t find this PC, allow that port in your firewall.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .body { padding: 0 32px 32px; }
  .lede { margin: 4px 0 20px; color: var(--muted); }
  .device { display: flex; align-items: center; gap: 12px; padding: 12px 0; border-bottom: 1px solid var(--line); }
  .glyph { width: 44px; height: 44px; border-radius: 50%; display: grid; place-items: center; background: var(--raised); color: var(--muted); flex: none; }
  .glyph.on { background: var(--accent-soft); color: var(--accent); }
  .info { flex: 1; display: grid; min-width: 0; }
  .info span { color: var(--muted); font-size: 14px; }
  .pair { width: 100%; min-height: 44px; margin-top: 20px; }
  .invite { display: grid; grid-template-columns: 200px 1fr; gap: 20px; align-items: start; margin-top: 20px; }
  .qr { background: #fff; padding: 10px; border-radius: 12px; line-height: 0; }
  .qr :global(svg) { width: 100%; height: auto; }
  .invite p { color: var(--muted); margin: 8px 0; }
  code { display: block; margin-bottom: 10px; padding: 8px; font-size: 11px; word-break: break-all; background: var(--raised); border-radius: 8px; user-select: all; }
  .problem { margin: 12px 0; padding: 12px; border-radius: 12px; color: var(--danger); background: var(--danger-soft); }
  .foot { margin: 20px 0 0; color: var(--muted); font-size: 13px; }
</style>
