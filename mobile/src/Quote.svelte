<script lang="ts">
  import type { Quote } from '../../src/lib/types';
  import { openUrl } from './api';

  let { quote }: { quote: Quote } = $props();

  function open(event: MouseEvent) {
    event.stopPropagation();
    if (quote.canonicalUrl) void openUrl(quote.canonicalUrl);
  }
</script>

<button type="button" class="quote" onclick={open}>
  <span class="head">
    {#if quote.authorName}<strong>{quote.authorName}</strong>{/if}
    {#if quote.username}<span>@{quote.username}</span>{/if}
  </span>
  {#if quote.text}<span class="text">{quote.text}</span>{/if}
</button>

<style>
  .quote {
    display: block;
    width: 100%;
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--line-strong);
    border-color: var(--line);
    border-radius: 16px;
    text-align: left;
  }
  .head { display: flex; gap: 4px; white-space: nowrap; overflow: hidden; margin-bottom: 2px; }
  .head strong { overflow: hidden; text-overflow: ellipsis; }
  .head span { color: var(--muted); overflow: hidden; text-overflow: ellipsis; }
  .text {
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
