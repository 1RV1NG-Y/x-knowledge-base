<script lang="ts">
  import { inTauri } from '../api';

  // Drawn by the app (the system title bar is hidden); the OS still owns moving, snapping
  // and resizing the window.
  let maximized = $state(false);

  $effect(() => {
    if (!inTauri) return;
    let stop: (() => void) | undefined;
    let alive = true;
    void import('@tauri-apps/api/window').then(async ({ getCurrentWindow }) => {
      const win = getCurrentWindow();
      const sync = async () => (maximized = await win.isMaximized());
      await sync();
      const unlisten = await win.onResized(() => void sync());
      if (alive) stop = unlisten;
      else unlisten();
    });
    return () => {
      alive = false;
      stop?.();
    };
  });

  async function act(action: 'minimize' | 'toggleMaximize' | 'close') {
    if (!inTauri) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow()[action]();
  }
</script>

<div class="controls">
  <button type="button" aria-label="Minimize" title="Minimize" onclick={() => void act('minimize')}>
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6h7" /></svg>
  </button>
  <button type="button" aria-label={maximized ? 'Restore' : 'Maximize'} title={maximized ? 'Restore' : 'Maximize'} onclick={() => void act('toggleMaximize')}>
    {#if maximized}
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 4.5h5v5h-5zM4.5 4.5v-2h5v5h-2" /></svg>
    {:else}
      <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" rx="1" /></svg>
    {/if}
  </button>
  <button type="button" class="close" aria-label="Close" title="Close" onclick={() => void act('close')}>
    <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" /></svg>
  </button>
</div>

<style>
  .controls { display: flex; gap: 4px; padding: 0 6px; }
  button {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
    transition: background-color 120ms ease, color 120ms ease;
  }
  button:hover { background: var(--hover-strong); color: var(--text); }
  button.close:hover { background: var(--danger); color: #fff; }
  svg { width: 12px; height: 12px; fill: none; stroke: currentColor; stroke-width: 1.4; stroke-linecap: round; stroke-linejoin: round; }
</style>
