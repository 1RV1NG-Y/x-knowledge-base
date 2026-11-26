<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import { desk } from '../state.svelte';

  // The sticky, blurred bar at the top of the middle column.
  interface Props {
    title?: string;
    subtitle?: string;
    back?: boolean;
    actions?: Snippet;
    children?: Snippet;
  }
  let { title, subtitle, back = false, actions, children }: Props = $props();
</script>

<header class="bar">
  {#if title}
    <div class="top">
      {#if back}
        <button type="button" class="round" aria-label="Back" title="Back (Esc)" onclick={() => desk.back()}><Icon name="back" size={20} /></button>
      {/if}
      <div class="titles">
        <h1>{title}</h1>
        {#if subtitle}<span>{subtitle}</span>{/if}
      </div>
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </div>
  {/if}
  {#if children}{@render children()}{/if}
</header>

<style>
  .bar {
    position: sticky;
    top: 0;
    z-index: 5;
    background: var(--bg-blur);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--line);
  }
  .top { display: flex; align-items: center; gap: 24px; min-height: var(--header-row); padding: 0 16px; }
  .top .round { margin-left: -8px; }
  .titles { display: grid; min-width: 0; flex: 1; line-height: 20px; }
  h1 { font-size: 20px; line-height: 24px; font-weight: 700; margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .titles span { color: var(--muted); font-size: 13px; line-height: 16px; }
  .actions { display: flex; align-items: center; gap: 8px; }
</style>
