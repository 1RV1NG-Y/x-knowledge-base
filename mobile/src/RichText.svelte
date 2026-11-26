<script lang="ts">
  import { openUrl } from './api';
  import { segments } from './format';

  let { text, large = false, clamp = false }: { text: string; large?: boolean; clamp?: boolean } = $props();
  const parts = $derived(segments(text));
  // Long posts are cut short in timelines; the post's own page shows all of it.
  const long = $derived(text.length > 420 || text.split('\n').length > 9);

  function follow(event: MouseEvent, href?: string) {
    event.stopPropagation();
    if (href) void openUrl(href);
  }
</script>

<div class="text" class:large class:clamp={clamp && long}>
  {#each parts as part}{#if part.kind === 'text'}{part.text}{:else}<button type="button" class="link" onclick={(e) => follow(e, part.href)}>{part.text}</button>{/if}{/each}
</div>
{#if clamp && long}<span class="show-more">Show more</span>{/if}

<style>
  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 15px;
    line-height: 20px;
  }
  .text.large { font-size: 17px; line-height: 24px; }
  .text.clamp {
    display: -webkit-box;
    -webkit-line-clamp: 8;
    line-clamp: 8;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .show-more { color: var(--accent); display: inline-block; margin-top: 2px; }
  .link {
    color: var(--accent);
    display: inline;
    font-size: inherit;
    line-height: inherit;
    text-align: left;
  }
</style>
