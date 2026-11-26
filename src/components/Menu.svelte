<script lang="ts">
  import { onMount, tick, type Snippet } from 'svelte';

  // A floating menu next to `anchor`; closes on a click outside, Escape or scrolling.
  interface Props {
    anchor: HTMLElement;
    onclose: () => void;
    align?: 'start' | 'end';
    width?: number;
    label?: string;
    children: Snippet;
  }

  let { anchor, onclose, align = 'end', width, label, children }: Props = $props();
  let menu = $state<HTMLDivElement>();
  let style = $state('visibility: hidden');

  async function place() {
    await tick();
    if (!menu) return;
    const a = anchor.getBoundingClientRect();
    const m = menu.getBoundingClientRect();
    const gap = 4;
    let top = a.bottom + gap;
    if (top + m.height > window.innerHeight - 8 && a.top - gap - m.height > 40) top = a.top - gap - m.height;
    top = Math.max(44, Math.min(top, window.innerHeight - m.height - 8));
    let left = align === 'end' ? a.right - m.width : a.left;
    left = Math.max(8, Math.min(left, window.innerWidth - m.width - 8));
    style = `top:${top}px;left:${left}px;${width ? `width:${width}px;` : ''}`;
  }

  onMount(() => {
    if (width) style = `visibility:hidden;width:${width}px`;
    void place();
    const outside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!menu?.contains(target) && !anchor.contains(target)) onclose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.stopPropagation();
        onclose();
        anchor.focus();
      }
    };
    const scroll = (event: Event) => {
      if (!menu?.contains(event.target as Node)) onclose();
    };
    document.addEventListener('pointerdown', outside, true);
    window.addEventListener('keydown', key, true);
    document.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', onclose);
    return () => {
      document.removeEventListener('pointerdown', outside, true);
      window.removeEventListener('keydown', key, true);
      document.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', onclose);
    };
  });
</script>

<!-- Clicks inside stay inside: a menu on a post must not open the post. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="popover" role="menu" tabindex="-1" aria-label={label} bind:this={menu} {style} onclick={(e) => e.stopPropagation()}>
  {@render children()}
</div>
