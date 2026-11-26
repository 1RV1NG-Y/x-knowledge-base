<script lang="ts">
  import Icon from '../../mobile/src/Icon.svelte';

  // A list's colored square, the same one the phone shows.
  let { id, size = 48 }: { id: number; size?: number } = $props();
  const hue = $derived((id * 67) % 360);
</script>

<span
  class="tile"
  style:width="{size}px"
  style:height="{size}px"
  style:background="linear-gradient(135deg, hsl({hue} 70% 55%), hsl({hue + 40} 70% 40%))"
><Icon name="lists" size={Math.round(size * 0.46)} /></span>

<style>
  .tile { border-radius: 12px; display: grid; place-items: center; color: #fff; flex: none; }
</style>
