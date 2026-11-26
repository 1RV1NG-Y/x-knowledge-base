<script lang="ts">
  import Icon from './Icon.svelte';

  interface Tab {
    id: string;
    label: string;
    /** Tapping the selected tab opens a menu: show a chevron while it's selected. */
    menu?: boolean;
    /** A dot for "options in use". */
    badge?: boolean;
  }

  let { tabs, value, onchange }: { tabs: Tab[]; value: string; onchange: (id: string) => void } = $props();
</script>

<div class="tabs" role="tablist">
  {#each tabs as tab (tab.id)}
    <button
      type="button"
      role="tab"
      data-tab={tab.id}
      aria-selected={value === tab.id}
      aria-haspopup={tab.menu && value === tab.id ? 'dialog' : undefined}
      class:on={value === tab.id}
      onclick={() => onchange(tab.id)}
    >
      <span class="label">
        {tab.label}
        {#if tab.menu && value === tab.id}<Icon name="chevronDown" size={16} />{/if}
        {#if tab.badge}<span class="badge" aria-label="filtered"></span>{/if}
      </span>
    </button>
  {/each}
</div>

<style>
  .tabs { display: flex; height: var(--tabs-row); }
  button {
    flex: 1;
    display: flex;
    justify-content: center;
    color: var(--muted);
    font-weight: 500;
  }
  button:active { background: var(--hover); }
  /* Desktop (shares this component). */
  @media (hover: hover) and (pointer: fine) {
    button { transition: background-color 120ms ease; }
    button:hover { background: var(--hover-strong, var(--hover)); }
  }
  .label {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 100%;
  }
  .on { color: var(--text); font-weight: 700; }
  .on .label::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    border-radius: 9999px;
    background: var(--accent);
  }
  .badge { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); margin-left: 2px; }
</style>
