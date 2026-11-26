<script lang="ts">
  import { onMount } from 'svelte';
  import type { Media } from '../../src/lib/types';
  import { fileUrl } from './api';
  import Icon from './Icon.svelte';
  import { app } from './state.svelte';

  let { media, index }: { media: Media[]; index: number } = $props();

  let track: HTMLElement;
  let current = $state(0);
  let chrome = $state(true);
  // Zoom of the current slide.
  let scale = $state(1);
  let x = $state(0);
  let y = $state(0);
  const pointers = new Map<number, { x: number; y: number }>();
  let pinch = $state<{ distance: number; scale: number }>();
  let pan = $state<{ x: number; y: number; ox: number; oy: number }>();
  let lastTap = 0;
  let moved = false;

  const src = (item: Media) =>
    item.kind === 'image'
      ? fileUrl(item.localPath) ?? item.remoteUrl ?? item.previewUrl
      : fileUrl(item.thumbnailPath) ?? item.previewUrl;

  function reset() {
    scale = 1;
    x = 0;
    y = 0;
  }

  function onscroll() {
    const next = Math.round(track.scrollLeft / track.clientWidth);
    if (next !== current) {
      current = next;
      reset();
    }
  }

  const distance = () => {
    const [a, b] = [...pointers.values()];
    return a && b ? Math.hypot(a.x - b.x, a.y - b.y) : 0;
  };

  function down(event: PointerEvent) {
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    moved = false;
    if (pointers.size === 2) {
      pinch = { distance: distance(), scale };
      pan = undefined;
    } else if (pointers.size === 1 && scale > 1) {
      pan = { x: event.clientX, y: event.clientY, ox: x, oy: y };
    }
  }

  function move(event: PointerEvent) {
    if (!pointers.has(event.pointerId)) return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    if (pinch && pointers.size === 2) {
      moved = true;
      scale = Math.min(5, Math.max(1, (pinch.scale * distance()) / pinch.distance));
      if (scale === 1) { x = 0; y = 0; }
    } else if (pan) {
      const dx = event.clientX - pan.x;
      const dy = event.clientY - pan.y;
      if (Math.abs(dx) + Math.abs(dy) > 4) moved = true;
      x = pan.ox + dx;
      y = pan.oy + dy;
    }
  }

  function up(event: PointerEvent) {
    pointers.delete(event.pointerId);
    if (pointers.size < 2) pinch = undefined;
    if (pointers.size === 0) {
      pan = undefined;
      if (!moved) tap();
    }
  }

  function tap() {
    const now = Date.now();
    if (now - lastTap < 280) {
      // Double tap: zoom in or back out.
      if (scale > 1) reset();
      else scale = 2.5;
      lastTap = 0;
    } else {
      lastTap = now;
      setTimeout(() => {
        if (lastTap === now) chrome = !chrome;
      }, 290);
    }
  }

  onMount(() => {
    current = index;
    track.scrollLeft = index * track.clientWidth;
  });
</script>

<div class="viewer" role="dialog" aria-modal="true" aria-label="Media">
  <div class="track" bind:this={track} {onscroll} class:locked={scale > 1}>
    {#each media as item, i (item.id)}
      <div
        class="slide"
        role="presentation"
        onpointerdown={down}
        onpointermove={move}
        onpointerup={up}
        onpointercancel={up}
        style:touch-action={scale > 1 ? 'none' : 'pan-x'}
      >
        {#if item.kind !== 'image' && item.remoteUrl}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video src={item.remoteUrl} poster={src(item)} controls playsinline></video>
        {:else}
          <img
            src={src(item)}
            alt=""
            draggable="false"
            style:transform={i === current ? `translate(${x}px, ${y}px) scale(${scale})` : undefined}
            class:animate={!pinch && !pan}
          />
        {/if}
      </div>
    {/each}
  </div>
  {#if chrome}
    <div class="top">
      <button type="button" class="round" aria-label="Close" onclick={() => app.pop()}><Icon name="close" size={20} /></button>
      {#if media.length > 1}<span class="count">{current + 1} / {media.length}</span>{/if}
    </div>
  {/if}
</div>

<style>
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: #000;
    color: #fff;
  }
  .track {
    position: absolute;
    inset: 0;
    display: flex;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }
  .track::-webkit-scrollbar { display: none; }
  .track.locked { overflow: hidden; }
  .slide {
    flex: 0 0 100%;
    height: 100%;
    scroll-snap-align: center;
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  img, video { max-width: 100%; max-height: 100%; object-fit: contain; user-select: none; }
  img.animate { transition: transform 180ms ease; }
  .top {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 12px;
  }
  .top .round { background: rgb(15 20 25 / 0.75); color: #fff; }
  .count { font-weight: 700; }
</style>
