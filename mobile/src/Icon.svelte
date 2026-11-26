<script lang="ts">
  import { icons, strokeIcons, type IconName, type StrokeIconName } from './icons';

  let { name, size = 24, label }: { name: IconName | StrokeIconName; size?: number; label?: string } = $props();
  const stroke = $derived(name in strokeIcons);
</script>

<svg
  class="icon"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  style:width="{size}px"
  style:height="{size}px"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
>
  {#if stroke}
    <path d={strokeIcons[name as StrokeIconName]} fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
  {:else}
    <path d={icons[name as IconName]} />
  {/if}
</svg>
