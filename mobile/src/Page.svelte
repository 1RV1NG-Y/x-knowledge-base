<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    header?: Snippet;
    children: Snippet;
    /** Called when the end of the content comes near (load the next page). */
    onend?: () => void;
    /** Hide the header while scrolling down, show it again on the way up. */
    autohide?: boolean;
    active?: boolean;
    /** Leave room at the bottom for the tab bar. */
    nav?: boolean;
    onchrome?: (hidden: boolean) => void;
    /** Pull down at the top to call this; the spinner stays until it settles. */
    onrefresh?: () => Promise<unknown>;
    scroller?: HTMLElement;
  }

  let { header, children, onend, autohide = true, active = true, nav = true, onchrome, onrefresh, scroller = $bindable() }: Props = $props();

  // Pull to refresh.
  const TRIGGER = 64;
  const HOLD = 52;
  let pull = $state(0);
  let dragging = $state(false);
  let refreshing = $state(false);
  let startY: number | undefined;

  function touchstart(event: TouchEvent) {
    if (!onrefresh || refreshing || !scroller || scroller.scrollTop > 0 || event.touches.length > 1) return;
    startY = event.touches[0]!.clientY;
  }

  function touchmove(event: TouchEvent) {
    if (startY == null || !scroller) return;
    const dy = event.touches[0]!.clientY - startY;
    if (scroller.scrollTop > 0 || dy < -8) {
      // Scrolling the list, not pulling.
      startY = undefined;
      dragging = false;
      pull = 0;
      return;
    }
    dragging = dy > 0;
    // Resistance: the further you pull, the less it follows.
    pull = dy > 0 ? Math.min(120, dy * 0.5) : 0;
  }

  async function touchend() {
    if (startY == null) return;
    startY = undefined;
    dragging = false;
    if (pull < TRIGGER || !onrefresh) {
      pull = 0;
      return;
    }
    refreshing = true;
    pull = HOLD;
    try {
      await Promise.all([onrefresh(), new Promise((r) => setTimeout(r, 450))]);
    } finally {
      refreshing = false;
      pull = 0;
    }
  }

  let hidden = $state(false);
  let headerHeight = $state(0);
  let last = 0;
  let sentinel = $state<HTMLElement>();

  function onscroll() {
    if (!scroller || !autohide) return;
    const y = scroller.scrollTop;
    const delta = y - last;
    let next = hidden;
    if (y <= headerHeight) next = false;
    else if (delta > 8) next = true;
    else if (delta < -8) next = false;
    last = y;
    if (next !== hidden) {
      hidden = next;
      onchrome?.(hidden);
    }
  }

  $effect(() => {
    if (!sentinel || !scroller || !onend) return;
    const handler = onend;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) handler();
    }, { root: scroller, rootMargin: '0px 0px 1600px 0px' });
    observer.observe(sentinel);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (active) onchrome?.(hidden);
  });
</script>

<div class="page" class:inactive={!active} aria-hidden={!active}>
  {#if header}
    <header class="bar" class:hidden bind:clientHeight={headerHeight}>{@render header()}</header>
  {/if}
  {#if onrefresh && (pull > 0 || refreshing)}
    <div
      class="puller"
      class:settling={!dragging}
      style:top="{headerHeight}px"
      style:transform="translate(-50%, {pull - 40}px)"
      style:opacity={Math.min(1, pull / TRIGGER)}
      role={refreshing ? 'status' : undefined}
      aria-label={refreshing ? 'Refreshing' : undefined}
    >
      {#if refreshing}
        <span class="spin"></span>
      {:else}
        <span class="arrow" class:ready={pull >= TRIGGER}><Icon name="arrowDown" size={20} /></span>
      {/if}
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_static_element_interactions (touch only drives pull to refresh) -->
  <div
    class="scroller"
    bind:this={scroller}
    {onscroll}
    ontouchstart={touchstart}
    ontouchmove={touchmove}
    ontouchend={touchend}
    ontouchcancel={touchend}
    style:padding-top="{headerHeight}px"
    class:nav
  >
    <div class="content" class:settling={!dragging} style:transform={pull > 0 ? `translateY(${pull}px)` : undefined}>
      {@render children()}
    </div>
    {#if onend}<div class="sentinel" bind:this={sentinel}></div>{/if}
  </div>
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
    background: var(--bg);
  }
  .page.inactive {
    visibility: hidden;
    pointer-events: none;
  }
  .bar {
    position: absolute;
    z-index: 2;
    top: 0;
    left: 0;
    right: 0;
    background: var(--bg-blur);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--line);
    transition: transform 220ms ease;
  }
  .bar.hidden { transform: translateY(-100%); }
  .scroller {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: none;
  }
  .scroller::-webkit-scrollbar { display: none; }
  .scroller.nav { padding-bottom: calc(var(--nav-h) + 24px); }
  .sentinel { height: 1px; }
  .content.settling { transition: transform 200ms ease; }
  .puller {
    position: absolute;
    z-index: 1;
    left: 50%;
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    color: var(--accent);
  }
  .puller.settling { transition: transform 200ms ease, opacity 200ms ease; }
  .arrow { display: grid; transition: transform 160ms ease; }
  .arrow.ready { transform: rotate(180deg); }
  .spin {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2.5px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
