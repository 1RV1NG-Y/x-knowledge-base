<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { loadVideo, playVideoInMpv } from '../api';
  import type { Media } from '../types';

  interface Props { media: Media[] }
  let { media }: Props = $props();

  let videoSources = $state<Record<string, string>>({});
  let loadingVideos = $state<Set<string>>(new Set());
  let videoErrors = $state<Record<string, string>>({});
  let mpvLoading = $state<Set<string>>(new Set());
  let mpvErrors = $state<Record<string, string>>({});
  let viewer = $state<{ source: string; fallback?: string }>();
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let dragPointerId = $state<number>();
  let closeButton = $state<HTMLButtonElement>();
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  const localUrl = (path?: string): string | undefined => path ? convertFileSrc(path) : undefined;
  const imageSource = (item: Media): string | undefined => localUrl(item.localPath) ?? item.remoteUrl ?? item.previewUrl;
  const videoUrl = (item: Media): string | undefined => item.remoteUrl;
  const preview = (item: Media): string | undefined => localUrl(item.thumbnailPath) ?? item.previewUrl;

  function useImageFallback(event: Event, fallback?: string): void {
    const element = event.currentTarget as HTMLImageElement;
    if (fallback && element.src !== fallback) element.src = fallback;
  }

  async function loadEmbeddedVideo(item: Media): Promise<void> {
    if (videoSources[item.id] || loadingVideos.has(item.id)) return;
    loadingVideos = new Set(loadingVideos).add(item.id);
    const nextErrors = { ...videoErrors };
    delete nextErrors[item.id];
    videoErrors = nextErrors;

    try {
      const bytes = await loadVideo(item.id);
      if (bytes.byteLength === 0) throw new Error('Video response was empty.');
      videoSources = {
        ...videoSources,
        [item.id]: URL.createObjectURL(new Blob([bytes], { type: 'video/mp4' }))
      };
    } catch (cause) {
      videoErrors = {
        ...videoErrors,
        [item.id]: cause instanceof Error ? cause.message : String(cause)
      };
    } finally {
      const nextLoading = new Set(loadingVideos);
      nextLoading.delete(item.id);
      loadingVideos = nextLoading;
    }
  }

  async function playInMpv(item: Media): Promise<void> {
    if (mpvLoading.has(item.id)) return;
    mpvLoading = new Set(mpvLoading).add(item.id);
    const nextErrors = { ...mpvErrors };
    delete nextErrors[item.id];
    mpvErrors = nextErrors;

    try {
      await playVideoInMpv(item.id);
    } catch (cause) {
      mpvErrors = {
        ...mpvErrors,
        [item.id]: cause instanceof Error ? cause.message : String(cause)
      };
    } finally {
      const nextLoading = new Set(mpvLoading);
      nextLoading.delete(item.id);
      mpvLoading = nextLoading;
    }
  }

  async function openViewer(item: Media, event: MouseEvent): Promise<void> {
    const image = (event.currentTarget as HTMLButtonElement).querySelector('img');
    const source = image?.currentSrc || imageSource(item);
    if (!source) return;
    viewer = { source, fallback: item.remoteUrl ?? item.previewUrl };
    zoom = 1;
    panX = 0;
    panY = 0;
    document.body.style.overflow = 'hidden';
    await tick();
    closeButton?.focus();
  }

  function closeViewer(): void {
    viewer = undefined;
    zoom = 1;
    panX = 0;
    panY = 0;
    dragPointerId = undefined;
    document.body.style.overflow = '';
  }

  function setZoom(nextZoom: number, originX = 0, originY = 0): void {
    const clamped = Math.min(8, Math.max(1, nextZoom));
    if (clamped === 1) {
      zoom = 1;
      panX = 0;
      panY = 0;
      return;
    }
    const ratio = clamped / zoom;
    panX = originX - (originX - panX) * ratio;
    panY = originY - (originY - panY) * ratio;
    zoom = clamped;
  }

  function zoomWithWheel(event: WheelEvent): void {
    event.preventDefault();
    const stage = event.currentTarget as HTMLElement;
    const bounds = stage.getBoundingClientRect();
    setZoom(
      zoom * (event.deltaY < 0 ? 1.18 : 1 / 1.18),
      event.clientX - bounds.left - bounds.width / 2,
      event.clientY - bounds.top - bounds.height / 2
    );
  }

  function startPan(event: PointerEvent): void {
    if (zoom === 1) return;
    dragPointerId = event.pointerId;
    dragOffsetX = event.clientX - panX;
    dragOffsetY = event.clientY - panY;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function movePan(event: PointerEvent): void {
    if (dragPointerId !== event.pointerId) return;
    panX = event.clientX - dragOffsetX;
    panY = event.clientY - dragOffsetY;
  }

  function stopPan(event: PointerEvent): void {
    if (dragPointerId === event.pointerId) dragPointerId = undefined;
  }

  function handleViewerKeydown(event: KeyboardEvent): void {
    if (!viewer) return;
    if (['Escape', '+', '=', '-', '0'].includes(event.key)) event.stopPropagation();
    if (event.key === 'Escape') closeViewer();
    else if (event.key === '+' || event.key === '=') setZoom(zoom * 1.25);
    else if (event.key === '-') setZoom(zoom / 1.25);
    else if (event.key === '0') setZoom(1);
  }

  onDestroy(() => {
    for (const source of Object.values(videoSources)) URL.revokeObjectURL(source);
    document.body.style.overflow = '';
  });
</script>

{#if media.length}
  <div class:single={media.length === 1} class="media-grid" aria-label={`${media.length} media attachment${media.length === 1 ? '' : 's'}`}>
    {#each media.slice(0, 4) as item (item.id)}
      <figure>
        {#if item.kind === 'video'}
          {#if videoSources[item.id]}
            <video
              controls
              autoplay
              muted
              playsinline
              preload="auto"
              poster={preview(item)}
              aria-label="Video attachment; starts muted"
              src={videoSources[item.id]}
            >
              Your browser cannot play this video.
            </video>
          {:else if preview(item)}
            <img
              src={preview(item)}
              alt="Video preview"
              loading="lazy"
              onerror={(event) => useImageFallback(event, item.previewUrl)}
            />
          {:else}
            <div class="missing-media">Video preview unavailable</div>
          {/if}
          {#if videoUrl(item)}
            {#if !videoSources[item.id]}
              <button
                class="video-play"
                type="button"
                disabled={loadingVideos.has(item.id)}
                onclick={() => void loadEmbeddedVideo(item)}
              >{loadingVideos.has(item.id) ? 'Loading video…' : videoErrors[item.id] ? 'Retry video' : 'Play video'}</button>
            {/if}
            <button
              class="video-link"
              type="button"
              disabled={mpvLoading.has(item.id)}
              aria-label="Play video in mpv"
              onclick={() => void playInMpv(item)}
            >{mpvLoading.has(item.id) ? 'Opening mpv…' : 'Play in mpv'}</button>
            {#if videoErrors[item.id] || mpvErrors[item.id]}
              <span class="video-error" title={videoErrors[item.id] ?? mpvErrors[item.id]}>
                {videoErrors[item.id] ? 'Could not load video' : 'Could not start mpv'}
              </span>
            {/if}
          {:else}
            <span class="video-badge">Video unavailable</span>
          {/if}
        {:else if imageSource(item)}
          <button
            class="image-open"
            type="button"
            aria-label="Open image viewer"
            onclick={(event) => void openViewer(item, event)}
          >
            <img
              src={imageSource(item)}
              alt="Post attachment"
              loading="lazy"
              width={item.width}
              height={item.height}
              onerror={(event) => useImageFallback(event, item.remoteUrl ?? item.previewUrl)}
            />
          </button>
        {:else}
          <div class="missing-media">Image unavailable</div>
        {/if}
        {#if media.length > 4 && item === media[3]}<span class="more-media">+{media.length - 4}</span>{/if}
      </figure>
    {/each}
  </div>
{/if}

{#if viewer}
  <div
    class="image-viewer"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Image viewer"
    onclick={(event) => { if (event.target === event.currentTarget) closeViewer(); }}
    onkeydown={handleViewerKeydown}
  >
    <div class="image-viewer-toolbar">
      <span>{Math.round(zoom * 100)}%</span>
      <button type="button" aria-label="Zoom out" title="Zoom out" onclick={() => setZoom(zoom / 1.25)}>−</button>
      <button type="button" aria-label="Reset zoom" title="Reset zoom" onclick={() => setZoom(1)}>Reset</button>
      <button type="button" aria-label="Zoom in" title="Zoom in" onclick={() => setZoom(zoom * 1.25)}>+</button>
      <button class="image-viewer-close" bind:this={closeButton} type="button" aria-label="Close image viewer" title="Close" onclick={closeViewer}>×</button>
    </div>
    <div
      class:zoomed={zoom > 1}
      class:dragging={dragPointerId !== undefined}
      class="image-viewer-stage"
      role="application"
      aria-label="Zoomable image canvas"
      onwheel={zoomWithWheel}
      onpointerdown={startPan}
      onpointermove={movePan}
      onpointerup={stopPan}
      onpointercancel={stopPan}
      ondblclick={() => setZoom(zoom === 1 ? 2 : 1)}
    >
      <img
        src={viewer.source}
        alt="Expanded post attachment"
        draggable="false"
        style:transform={`translate(${panX}px, ${panY}px) scale(${zoom})`}
        onerror={(event) => useImageFallback(event, viewer?.fallback)}
      />
    </div>
    <p class="image-viewer-hint">Scroll to zoom · drag to pan · Esc to close</p>
  </div>
{/if}

<svelte:window onkeydown={handleViewerKeydown} />
