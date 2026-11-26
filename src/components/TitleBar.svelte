<script lang="ts">
  import { desk } from '../state.svelte';
  import WindowControls from './WindowControls.svelte';

  // The window's own header. Everything but the buttons is a drag region: drag moves the
  // window through the window manager (so snapping works) and double-click maximizes.
  const job = $derived(desk.job);
  const working = $derived(Boolean(job?.total && job.processed < job.total));
  const percent = $derived(job?.total ? Math.min(100, (job.processed / job.total) * 100) : 0);
</script>

<header class="titlebar" data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>X Knowledge Base</span>
  <div class="spacer" data-tauri-drag-region></div>
  {#if working && job}
    <span class="job" data-tauri-drag-region title={job.message || job.phase}>
      <span class="bar" data-tauri-drag-region><span style:width="{percent}%"></span></span>
      {job.phase || 'Working'} · {job.processed.toLocaleString()} of {job.total.toLocaleString()}
    </span>
  {/if}
  <WindowControls />
</header>

<style>
  .titlebar {
    position: relative;
    z-index: 30;
    height: var(--titlebar);
    display: flex;
    align-items: center;
    padding-left: 14px;
    background: var(--bg);
    user-select: none;
    -webkit-user-select: none;
  }
  .title { color: var(--muted); font-size: 13px; font-weight: 600; }
  .spacer { flex: 1; align-self: stretch; }
  .job { display: flex; align-items: center; gap: 8px; color: var(--muted); font-size: 12px; margin-right: 8px; }
  .bar { width: 80px; height: 4px; border-radius: 2px; background: var(--accent-soft); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); transition: width 300ms linear; }
</style>
