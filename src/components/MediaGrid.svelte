<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { Media } from '../lib/types';
  import Icon from '../../mobile/src/Icon.svelte';
  import { duration } from '../../mobile/src/format';
  import { fileUrl, loadVideo, playVideoInMpv } from '../api';
  import { desk, message } from '../state.svelte';

  // Photos open the viewer; videos play right here (from the saved copy when there is one)
  // or in mpv.
  let { media }: { media: Media[] } = $props();

  const items = $derived(media.slice(0, 4));
  const images = $derived(media.filter((m) => m.kind === 'image'));
  let failed = $state<Record<string, number>>({});
  let playing = $state<Record<string, string>>({});
  let busy = $state<Record<string, 'here' | 'mpv'>>({});
  let problems = $state<Record<string, string>>({});

  function sources(item: Media): string[] {
    const list = item.kind === 'image'
      ? [fileUrl(item.thumbnailPath), fileUrl(item.localPath), item.previewUrl, item.remoteUrl]
      : [fileUrl(item.thumbnailPath), item.previewUrl];
    return list.filter(Boolean) as string[];
  }
  const source = (item: Media) => sources(item)[failed[item.id] ?? 0];

  const single = $derived(items.length === 1 ? items[0] : undefined);
  const ratio = $derived.by(() => {
    if (!single?.width || !single?.height) return 16 / 9;
    return Math.min(Math.max(single.width / single.height, 0.75), 2.2);
  });

  function stop(event: Event) {
    event.stopPropagation();
  }

  function open(event: MouseEvent, item: Media) {
    stop(event);
    desk.viewer = { media: images, index: Math.max(0, images.indexOf(item)) };
  }

  async function playHere(event: MouseEvent, item: Media) {
    stop(event);
    if (busy[item.id]) return;
    busy = { ...busy, [item.id]: 'here' };
    problems = { ...problems, [item.id]: '' };
    try {
      const bytes = await loadVideo(item.id);
      if (bytes.byteLength === 0) throw new Error('The video came back empty.');
      playing = { ...playing, [item.id]: URL.createObjectURL(new Blob([bytes], { type: 'video/mp4' })) };
    } catch (cause) {
      problems = { ...problems, [item.id]: `Couldn’t load the video: ${message(cause)}` };
    } finally {
      const { [item.id]: _, ...rest } = busy;
      busy = rest;
    }
  }

  async function playInMpv(event: MouseEvent, item: Media) {
    stop(event);
    if (busy[item.id]) return;
    busy = { ...busy, [item.id]: 'mpv' };
    problems = { ...problems, [item.id]: '' };
    try {
      await playVideoInMpv(item.id);
    } catch (cause) {
      problems = { ...problems, [item.id]: `Couldn’t start mpv: ${message(cause)}` };
    } finally {
      const { [item.id]: _, ...rest } = busy;
      busy = rest;
    }
  }

  onDestroy(() => Object.values(playing).forEach((url) => URL.revokeObjectURL(url)));
</script>

{#if items.length}
  <div class="grid n{items.length}" style:aspect-ratio={items.length === 1 ? ratio : 16 / 9}>
    {#each items as item (item.id)}
      <div class="cell">
        {#if playing[item.id]}
          <!-- svelte-ignore a11y_media_has_caption -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <video src={playing[item.id]} poster={source(item)} controls autoplay playsinline onclick={stop}></video>
        {:else if item.kind === 'image'}
          <button type="button" class="tile" aria-label="Open image" onclick={(e) => open(e, item)}>
            {#if source(item)}
              <img src={source(item)} alt="" loading="lazy" decoding="async" onerror={() => (failed = { ...failed, [item.id]: (failed[item.id] ?? 0) + 1 })} />
            {/if}
          </button>
        {:else}
          <div class="tile">
            {#if source(item)}
              <img src={source(item)} alt="" loading="lazy" decoding="async" onerror={() => (failed = { ...failed, [item.id]: (failed[item.id] ?? 0) + 1 })} />
            {/if}
            {#if item.remoteUrl}
              <button type="button" class="play" aria-label="Play video" title="Play here" disabled={Boolean(busy[item.id])} onclick={(e) => void playHere(e, item)}>
                {#if busy[item.id] === 'here'}<span class="mini"></span>{:else}<Icon name="play" size={24} />{/if}
              </button>
              <button type="button" class="mpv" title="Play in mpv" disabled={Boolean(busy[item.id])} onclick={(e) => void playInMpv(e, item)}>
                {busy[item.id] === 'mpv' ? 'Opening…' : 'mpv'}
              </button>
            {:else}
              <span class="badge right">Video unavailable</span>
            {/if}
            {#if (item.kind as string) === 'gif'}
              <span class="badge">GIF</span>
            {:else if item.durationMs}
              <span class="badge">{duration(item.durationMs)}</span>
            {/if}
          </div>
        {/if}
        {#if problems[item.id]}<span class="problem" title={problems[item.id]}>{problems[item.id]}</span>{/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .grid {
    margin-top: 12px;
    display: grid;
    gap: 2px;
    border-radius: 16px;
    overflow: hidden;
    border: 1px solid var(--line);
    width: 100%;
    max-height: 516px;
    background: var(--line);
  }
  .n1 { grid-template: 1fr / 1fr; }
  .n2 { grid-template: 1fr / 1fr 1fr; }
  .n3 { grid-template: 1fr 1fr / 1fr 1fr; }
  .n3 .cell:first-child { grid-row: span 2; }
  .n4 { grid-template: 1fr 1fr / 1fr 1fr; }
  .cell { position: relative; min-width: 0; min-height: 0; background: var(--skeleton); }
  .tile { position: absolute; inset: 0; display: block; width: 100%; height: 100%; }
  button.tile { cursor: zoom-in; }
  button.tile:hover img { filter: brightness(0.94); }
  img, video { width: 100%; height: 100%; object-fit: cover; display: block; }
  video { background: #000; object-fit: contain; }
  .play {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 58px;
    height: 58px;
    margin: -29px 0 0 -29px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    background: var(--accent);
    border: 4px solid #fff;
    padding-left: 3px;
    transition: transform 120ms ease;
  }
  .play:hover { transform: scale(1.06); }
  .mpv {
    position: absolute;
    right: 10px;
    bottom: 10px;
    padding: 2px 8px;
    border-radius: 9999px;
    font-size: 13px;
    font-weight: 700;
    color: #fff;
    background: rgb(0 0 0 / 0.77);
  }
  .mpv:hover { background: rgb(0 0 0 / 0.9); }
  .badge {
    position: absolute;
    left: 10px;
    bottom: 10px;
    padding: 0 5px;
    border-radius: 4px;
    font-size: 13px;
    line-height: 18px;
    font-weight: 700;
    color: #fff;
    background: rgb(0 0 0 / 0.77);
  }
  .badge.right { left: auto; right: 10px; }
  .problem {
    position: absolute;
    left: 8px;
    right: 8px;
    top: 8px;
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    color: #fff;
    background: rgb(244 33 46 / 0.9);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mini { width: 20px; height: 20px; border-radius: 50%; border: 2.5px solid rgb(255 255 255 / 0.4); border-top-color: #fff; animation: spin 0.8s linear infinite; }
</style>
