<script lang="ts">
  import { onDestroy } from 'svelte';
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

  onDestroy(() => {
    for (const source of Object.values(videoSources)) URL.revokeObjectURL(source);
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
          <img
            src={imageSource(item)}
            alt="Post attachment"
            loading="lazy"
            width={item.width}
            height={item.height}
            onerror={(event) => useImageFallback(event, item.remoteUrl ?? item.previewUrl)}
          />
        {:else}
          <div class="missing-media">Image unavailable</div>
        {/if}
        {#if media.length > 4 && item === media[3]}<span class="more-media">+{media.length - 4}</span>{/if}
      </figure>
    {/each}
  </div>
{/if}
