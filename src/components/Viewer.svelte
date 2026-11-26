<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import { fileUrl } from '../api';
  import { desk } from '../state.svelte';

  // Full-window photos: arrows or ←/→ to move, wheel or +/− to zoom, drag to pan, Esc to close.
  const media = $derived(desk.viewer?.media ?? []);
  let index = $state(desk.viewer?.index ?? 0);
  let zoom = $state(1);
  let x = $state(0);
  let y = $state(0);
  let drag = $state<{ id: number; ox: number; oy: number }>();
  let failed = $state<Record<string, number>>({});

  const item = $derived(media[index]);
  const sources = $derived(item ? ([fileUrl(item.localPath), item.remoteUrl, item.previewUrl, fileUrl(item.thumbnailPath)].filter(Boolean) as string[]) : []);
  const source = $derived(item ? sources[failed[item.id] ?? 0] : undefined);

  const close = () => (desk.viewer = undefined);
  function reset() { zoom = 1; x = 0; y = 0; }
  function step(delta: number) {
    if (media.length < 2) return;
    index = (index + delta + media.length) % media.length;
    reset();
  }
  function setZoom(next: number, ox = 0, oy = 0) {
    const clamped = Math.min(8, Math.max(1, next));
    if (clamped === 1) return reset();
    const ratio = clamped / zoom;
    x = ox - (ox - x) * ratio;
    y = oy - (oy - y) * ratio;
    zoom = clamped;
  }
  function wheel(event: WheelEvent) {
    event.preventDefault();
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    setZoom(zoom * (event.deltaY < 0 ? 1.18 : 1 / 1.18), event.clientX - box.left - box.width / 2, event.clientY - box.top - box.height / 2);
  }
  function down(event: PointerEvent) {
    if (zoom === 1) return;
    drag = { id: event.pointerId, ox: event.clientX - x, oy: event.clientY - y };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }
  function move(event: PointerEvent) {
    if (drag?.id !== event.pointerId) return;
    x = event.clientX - drag.ox;
    y = event.clientY - drag.oy;
  }

  onMount(() => {
    const key = (e: KeyboardEvent) => {
      const handled = ['Escape', 'ArrowLeft', 'ArrowRight', '+', '=', '-', '0'].includes(e.key);
      if (!handled) return;
      e.stopPropagation();
      e.preventDefault();
      if (e.key === 'Escape') close();
      else if (e.key === 'ArrowLeft') step(-1);
      else if (e.key === 'ArrowRight') step(1);
      else if (e.key === '+' || e.key === '=') setZoom(zoom * 1.25);
      else if (e.key === '-') setZoom(zoom / 1.25);
      else reset();
    };
    window.addEventListener('keydown', key, true);
    return () => window.removeEventListener('keydown', key, true);
  });
</script>

<div class="viewer" role="dialog" aria-modal="true" aria-label="Photo">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="stage"
    class:zoomed={zoom > 1}
    onwheel={wheel}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={() => (drag = undefined)}
    onclick={(e) => { if (e.target === e.currentTarget && zoom === 1) close(); }}
    ondblclick={(e) => { if (zoom > 1) reset(); else { const b = e.currentTarget.getBoundingClientRect(); setZoom(2.5, e.clientX - b.left - b.width / 2, e.clientY - b.top - b.height / 2); } }}
  >
    {#if source && item}
      <img
        src={source}
        alt=""
        draggable="false"
        style:transform="translate({x}px, {y}px) scale({zoom})"
        class:animate={!drag}
        onerror={() => (failed = { ...failed, [item.id]: (failed[item.id] ?? 0) + 1 })}
      />
    {/if}
  </div>
  <button type="button" class="round close" aria-label="Close" title="Close (Esc)" onclick={close}><Icon name="close" size={20} /></button>
  {#if media.length > 1}
    <button type="button" class="round nav prev" aria-label="Previous" onclick={() => step(-1)}><Icon name="back" size={20} /></button>
    <button type="button" class="round nav next" aria-label="Next" onclick={() => step(1)}><Icon name="back" size={20} /></button>
    <span class="count">{index + 1} / {media.length}</span>
  {/if}
  {#if zoom > 1}<span class="zoom">{Math.round(zoom * 100)}%</span>{/if}
</div>

<style>
  .viewer { position: fixed; inset: var(--titlebar) 0 0; z-index: 60; background: rgb(0 0 0 / 0.92); color: #fff; animation: fade 140ms ease; }
  .stage { position: absolute; inset: 0; display: grid; grid-template: minmax(0, 1fr) / minmax(0, 1fr); place-items: center; overflow: hidden; cursor: zoom-in; }
  .stage.zoomed { cursor: grab; }
  img { max-width: 100%; max-height: 100%; object-fit: contain; user-select: none; }
  img.animate { transition: transform 160ms ease; }
  .round { position: absolute; width: 40px; height: 40px; color: #fff; background: rgb(15 20 25 / 0.75); backdrop-filter: blur(4px); }
  .round:hover { background: rgb(39 44 48 / 0.85); }
  .close { top: 12px; left: 12px; }
  .nav { top: 50%; margin-top: -20px; }
  .prev { left: 16px; }
  .next { right: 16px; transform: scaleX(-1); }
  .count, .zoom { position: absolute; bottom: 16px; padding: 4px 10px; border-radius: 9999px; background: rgb(15 20 25 / 0.75); font-size: 13px; font-weight: 700; }
  .count { left: 50%; transform: translateX(-50%); }
  .zoom { right: 16px; }
</style>
