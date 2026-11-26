<script lang="ts">
  import Icon from '../../mobile/src/Icon.svelte';
  import { desk } from '../state.svelte';

  // X's rounded search field. Typing shows results in the Search section.
  let { autofocus = false }: { autofocus?: boolean } = $props();
  let field = $state<HTMLInputElement>();

  function typed(value: string) {
    desk.search = value;
    if (value.trim() && (desk.section !== 'search' || desk.stack.length)) desk.go('search');
  }

  $effect(() => {
    if (autofocus) field?.focus();
  });
</script>

<label class="box">
  <Icon name="search" size={18} />
  <input
    bind:this={field}
    type="search"
    value={desk.search}
    oninput={(e) => typed(e.currentTarget.value)}
    onkeydown={(e) => { if (e.key === 'Enter') typed(e.currentTarget.value); }}
    placeholder="Search your likes"
    spellcheck="false"
    autocomplete="off"
    data-search
  />
  {#if desk.search}
    <button type="button" class="clear" aria-label="Clear search" onclick={() => { desk.search = ''; field?.focus(); }}><Icon name="close" size={12} /></button>
  {/if}
</label>

<style>
  .box {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    padding: 0 12px 0 16px;
    border-radius: 9999px;
    background: var(--raised);
    border: 1px solid transparent;
    color: var(--muted);
  }
  .box:focus-within { background: var(--bg); border-color: var(--accent); color: var(--accent); }
  input { flex: 1; min-width: 0; border: 0; outline: none; background: none; color: var(--text); font-size: 15px; }
  input::placeholder { color: var(--muted); }
  input::-webkit-search-cancel-button { display: none; }
  .clear { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; background: var(--accent); color: var(--bg); }
</style>
