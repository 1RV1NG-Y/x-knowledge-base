<script lang="ts">
  import type { Media } from '../../src/lib/types';
  import { fileUrl } from './api';
  import { duration } from './format';
  import Icon from './Icon.svelte';

  let { media, onopen }: { media: Media[]; onopen: (index: number) => void } = $props();

  const items = $derived(media.slice(0, 4));
  let playing = $state<string>();
  let failed = $state<Record<string, number>>({});

  // Saved copy first, then the online versions.
  function sources(item: Media): string[] {
    const list = item.kind === 'image'
      ? [fileUrl(item.localPath), item.remoteUrl, item.previewUrl]
      : [fileUrl(item.thumbnailPath), item.previewUrl];
    return list.filter(Boolean) as string[];
  }
  const source = (item: Media) => sources(item)[failed[item.id] ?? 0];

  const single = $derived(items.length === 1 ? items[0] : undefined);
  const ratio = $derived.by(() => {
    if (!single?.width || !single?.height) return 16 / 9;
    return Math.min(Math.max(single.width / single.height, 0.75), 2);
  });

  function open(event: MouseEvent, index: number, item: Media) {
    event.stopPropagation();
    if (item.kind === 'video' || item.kind === ('gif' as Media['kind'])) {
      if (item.remoteUrl) playing = item.id;
      return;
    }
    onopen(index);
  }
</script>

{#if items.length}
  <div class="grid n{items.length}" style:aspect-ratio={items.length === 1 ? ratio : 16 / 9}>
    {#each items as item, index (item.id)}
      <div class="cell">
        {#if playing === item.id && item.remoteUrl}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            src={item.remoteUrl}
            poster={source(item)}
            controls
            autoplay
            playsinline
            loop={(item.kind as string) === 'gif'}
            onclick={(e) => e.stopPropagation()}
          ></video>
        {:else}
          <button type="button" class="tile" onclick={(e) => open(e, index, item)} aria-label={item.kind === 'image' ? 'Open image' : 'Play video'}>
            {#if source(item)}
              <img
                src={source(item)}
                alt=""
                loading="lazy"
                decoding="async"
                onerror={() => (failed = { ...failed, [item.id]: (failed[item.id] ?? 0) + 1 })}
              />
            {/if}
            {#if item.kind !== 'image'}
              <span class="play"><Icon name="play" size={22} /></span>
              {#if (item.kind as string) === 'gif'}
                <span class="badge">GIF</span>
              {:else if item.durationMs}
                <span class="badge">{duration(item.durationMs)}</span>
              {/if}
            {/if}
          </button>
        {/if}
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
    max-height: 510px;
    background: var(--line);
  }
  .n1 { grid-template: 1fr / 1fr; }
  .n2 { grid-template: 1fr / 1fr 1fr; }
  .n3 { grid-template: 1fr 1fr / 1fr 1fr; }
  .n3 .cell:first-child { grid-row: span 2; }
  .n4 { grid-template: 1fr 1fr / 1fr 1fr; }
  .cell { position: relative; min-width: 0; min-height: 0; background: var(--skeleton); }
  .tile { position: absolute; inset: 0; display: block; width: 100%; height: 100%; }
  img, video { width: 100%; height: 100%; object-fit: cover; display: block; }
  video { background: #000; object-fit: contain; }
  .play {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 52px;
    height: 52px;
    margin: -26px 0 0 -26px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    background: var(--accent);
    border: 3px solid #fff;
    padding-left: 3px;
  }
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
</style>
