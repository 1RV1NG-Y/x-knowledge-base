<script lang="ts">
  import { fileUrl } from './api';

  let { path, url, name = '?', size = 40 }: { path?: string; url?: string; name?: string; size?: number } = $props();

  let failed = $state(0);
  // Local copy first, then the online one, then the initial.
  const sources = $derived([fileUrl(path), url].filter(Boolean) as string[]);
  const source = $derived(sources[failed]);
  const hue = $derived([...name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7));
</script>

{#if source}
  <img
    class="avatar"
    src={source}
    alt=""
    width={size}
    height={size}
    style:width="{size}px"
    style:height="{size}px"
    loading="lazy"
    decoding="async"
    onerror={() => (failed += 1)}
  />
{:else}
  <span
    class="avatar letter"
    style:width="{size}px"
    style:height="{size}px"
    style:font-size="{Math.round(size * 0.42)}px"
    style:background="hsl({hue} 45% 45%)"
    aria-hidden="true">{name.slice(0, 1).toUpperCase()}</span
  >
{/if}
