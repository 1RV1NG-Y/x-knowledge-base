<script lang="ts">
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import { setActiveProfile } from './api';
  import { app, message } from './state.svelte';

  let { onswitched }: { onswitched: () => void } = $props();

  const active = $derived(app.activeProfile);
  const others = $derived(app.profiles.profiles.filter((p) => p.id !== active?.id));

  async function choose(id: string) {
    try {
      await setActiveProfile(id);
      await app.refreshLibrary();
      app.pop();
      onswitched();
    } catch (cause) {
      app.showToast(message(cause));
    }
  }

  function go(tab: 'lists' | 'sync') {
    app.pop();
    app.tab = tab;
  }
</script>

<div class="scrim" role="presentation" onclick={() => app.pop()}></div>
<nav class="drawer" aria-label="Account">
  {#if active}
    <div class="me">
      <Avatar path={active.avatarLocalPath} url={active.avatarUrl} name={active.displayName} size={40} />
      <strong>{active.displayName}</strong>
      {#if active.username}<span class="muted">@{active.username}</span>{/if}
      <span class="count"><b>{active.postCount.toLocaleString()}</b> <span class="muted">liked posts</span></span>
    </div>
  {:else}
    <div class="me"><strong>No library yet</strong><span class="muted">Pair with your PC to bring it over.</span></div>
  {/if}
  <div class="menu">
    <button type="button" onclick={() => go('lists')}><Icon name="lists" size={24} />Lists</button>
    <button type="button" onclick={() => go('sync')}><Icon name="laptop" size={24} />Sync with PC</button>
  </div>
  {#if others.length}
    <p class="label">Switch account</p>
    {#each others as profile (profile.id)}
      <button type="button" class="account" onclick={() => void choose(profile.id)}>
        <Avatar path={profile.avatarLocalPath} url={profile.avatarUrl} name={profile.displayName} size={32} />
        <span><strong>{profile.displayName}</strong>{#if profile.username}<span class="muted">@{profile.username}</span>{/if}</span>
        <span class="muted small">{profile.postCount.toLocaleString()}</span>
      </button>
    {/each}
  {/if}
</nav>

<style>
  .scrim { position: fixed; inset: 0; z-index: 30; background: var(--scrim); animation: fade 160ms ease; }
  .drawer {
    position: fixed;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 31;
    width: min(280px, 80vw);
    overflow-y: auto;
    background: var(--bg);
    animation: slide 220ms ease;
    box-shadow: 0 0 20px rgb(0 0 0 / 0.25);
  }
  .me { display: grid; gap: 2px; padding: 16px; justify-items: start; }
  .me strong { margin-top: 8px; font-size: 17px; }
  .count { margin-top: 10px; font-size: 14px; }
  .muted { color: var(--muted); }
  .menu { border-top: 1px solid var(--line); padding: 8px 0; }
  .menu button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 14px 16px;
    font-size: 20px;
    line-height: 24px;
    font-weight: 700;
  }
  .menu button:active, .account:active { background: var(--hover); }
  .label { margin: 0; padding: 12px 16px 4px; color: var(--muted); font-size: 14px; border-top: 1px solid var(--line); }
  .account { width: 100%; display: flex; align-items: center; gap: 12px; padding: 10px 16px; text-align: left; }
  .account > span:nth-child(2) { display: grid; flex: 1; min-width: 0; line-height: 18px; }
  .account strong, .account span span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .small { font-size: 13px; }
  @keyframes slide { from { transform: translateX(-100%); } }
  @keyframes fade { from { opacity: 0; } }
</style>
