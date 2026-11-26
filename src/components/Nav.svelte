<script lang="ts">
  import Avatar from '../../mobile/src/Avatar.svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import { setActiveProfile } from '../api';
  import { importFrom } from '../importer';
  import { desk, message, type Section } from '../state.svelte';
  import Menu from './Menu.svelte';

  // The left column: sections, Phone, Import and the account switcher (X's layout).
  let { onhome }: { onhome: () => void } = $props();

  let importButton = $state<HTMLButtonElement>();
  let accountButton = $state<HTMLButtonElement>();
  let menu = $state<'import' | 'account'>();
  let switching = $state<string>();

  const items: { id: Section; label: string; icon: 'home' | 'search' | 'lists'; active: 'homeFill' | 'searchBold' | 'listsBold' }[] = [
    { id: 'home', label: 'Home', icon: 'home', active: 'homeFill' },
    { id: 'search', label: 'Search', icon: 'search', active: 'searchBold' },
    { id: 'lists', label: 'Lists', icon: 'lists', active: 'listsBold' }
  ];
  const progress = $derived(desk.sync?.status?.progress);
  const phoneBusy = $derived(Boolean(progress && ['checking', 'downloading', 'applying'].includes(progress.phase)));
  const me = $derived(desk.activeProfile);

  function go(id: Section) {
    if (id === desk.section && !desk.stack.length) {
      if (id === 'home') onhome();
      else desk.scrollTop();
      return;
    }
    desk.go(id);
  }

  async function switchTo(id: string) {
    if (switching || id === desk.profiles.activeProfileId) return;
    switching = id;
    try {
      await setActiveProfile(id);
      desk.profiles = { ...desk.profiles, activeProfileId: id };
      menu = undefined;
      desk.go('home');
      await Promise.all([desk.refreshProfiles(), desk.refreshPlaylists()]);
      desk.libraryVersion += 1;
    } catch (cause) {
      desk.showToast(`Couldn’t switch accounts: ${message(cause)}`);
    } finally {
      switching = undefined;
    }
  }
</script>

<nav class="nav" aria-label="Sections">
  <div class="stack">
    <button type="button" class="logo" aria-label="Home" title="X Knowledge Base" onclick={() => go('home')}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="5.5" y="3.5" width="12" height="15" rx="2.5" transform="rotate(-12 11.5 11)" opacity="0.45" />
        <rect x="6.5" y="5" width="12" height="15" rx="2.5" />
      </svg>
    </button>
    {#each items as item (item.id)}
      {@const on = desk.section === item.id}
      <button type="button" class="link" class:on aria-current={on ? 'page' : undefined} title={item.label} onclick={() => go(item.id)}>
        <span class="inner"><Icon name={on ? item.active : item.icon} size={26} /><span class="text">{item.label}</span></span>
      </button>
    {/each}
    <button type="button" class="link" title="Phone" onclick={() => (desk.phoneOpen = true)}>
      <span class="inner">
        <span class="icon-wrap">
          <Icon name="phone" size={26} />
          {#if phoneBusy}<span class="dot"></span>{/if}
        </span>
        <span class="text">Phone</span>
      </span>
    </button>
    <button type="button" class="pill primary big" bind:this={importButton} disabled={desk.importing} title="Import your X archive" onclick={() => (menu = menu === 'import' ? undefined : 'import')}>
      <span class="text">{desk.importing ? 'Importing…' : 'Import archive'}</span>
      <span class="glyph"><Icon name="download" size={24} /></span>
    </button>
  </div>

  {#if me}
    <button type="button" class="account" bind:this={accountButton} title="Accounts" onclick={() => (menu = menu === 'account' ? undefined : 'account')}>
      <Avatar path={me.avatarLocalPath} url={me.avatarUrl} name={me.displayName} size={40} />
      <span class="who text">
        <strong>{me.displayName}</strong>
        <span>{me.username ? `@${me.username}` : `${me.postCount.toLocaleString()} likes`}</span>
      </span>
      <span class="text more"><Icon name="more" size={18} /></span>
    </button>
  {/if}
</nav>

{#if menu === 'import' && importButton}
  <Menu anchor={importButton} onclose={() => (menu = undefined)} label="Import" align="start">
    <p class="label">Import the archive you downloaded from X. Importing again is safe; posts already here are kept.</p>
    <button type="button" class="item" onclick={() => { menu = undefined; void importFrom(false); }}><Icon name="zip" size={20} />ZIP file</button>
    <button type="button" class="item" onclick={() => { menu = undefined; void importFrom(true); }}><Icon name="folder" size={20} />Extracted folder</button>
  </Menu>
{/if}

{#if menu === 'account' && accountButton}
  <Menu anchor={accountButton} onclose={() => (menu = undefined)} label="Accounts" align="start" width={300}>
    {#each desk.profiles.profiles as profile (profile.id)}
      {@const on = profile.id === desk.profiles.activeProfileId}
      <button type="button" class="item acct" disabled={Boolean(switching)} onclick={() => void switchTo(profile.id)}>
        <Avatar path={profile.avatarLocalPath} url={profile.avatarUrl} name={profile.displayName} size={40} />
        <span class="who">
          <strong>{profile.displayName}</strong>
          <span>{profile.username ? `@${profile.username} · ` : ''}{profile.postCount.toLocaleString()} likes</span>
        </span>
        {#if on}<span class="tick"><Icon name="check" size={18} /></span>{:else if switching === profile.id}<span class="mini"></span>{/if}
      </button>
    {/each}
    <p class="label">Each imported archive is its own account.</p>
  </Menu>
{/if}

<style>
  .nav {
    position: sticky;
    top: 0;
    height: calc(100vh - var(--titlebar));
    width: var(--nav-w);
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 0 12px 12px;
  }
  .stack { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; }
  .logo { width: 52px; height: 52px; border-radius: 50%; display: grid; place-items: center; transition: background-color 120ms ease; }
  .logo:hover { background: var(--hover-strong); }
  .logo svg { width: 30px; height: 30px; fill: var(--text); }
  .link { padding: 4px 0; border-radius: 9999px; }
  .inner { display: flex; align-items: center; gap: 20px; padding: 12px; border-radius: 9999px; font-size: 20px; line-height: 24px; transition: background-color 120ms ease; }
  .link:hover .inner { background: var(--hover-strong); }
  .link.on { font-weight: 700; }
  .text { padding-right: 16px; }
  .icon-wrap { position: relative; display: grid; }
  .dot { position: absolute; top: -2px; right: -2px; width: 9px; height: 9px; border-radius: 50%; background: var(--accent); border: 2px solid var(--bg); }
  .big { width: 90%; min-height: 52px; margin-top: 16px; font-size: 17px; }
  .big .glyph { display: none; }
  .account {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px;
    border-radius: 9999px;
    text-align: left;
    transition: background-color 120ms ease;
  }
  .account:hover { background: var(--hover-strong); }
  .who { display: grid; min-width: 0; flex: 1; line-height: 20px; }
  .who strong, .who span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who span { color: var(--muted); font-weight: 400; }
  .account .text { padding: 0; }
  .more { display: grid; }
  .acct { gap: 12px; }
  .tick { color: var(--accent); display: grid; }
  .mini { width: 18px; height: 18px; border-radius: 50%; border: 2px solid var(--accent-soft); border-top-color: var(--accent); animation: spin 0.8s linear infinite; }

  /* Narrower windows: icons only, as on X. */
  @media (max-width: 1279px) {
    .nav { width: 88px; align-items: center; padding: 0 8px 12px; }
    .stack { align-items: center; }
    .text, .more { display: none; }
    .inner { gap: 0; }
    .big { width: 52px; height: 52px; min-height: 52px; padding: 0; }
    .big .glyph { display: grid; }
    .account { width: auto; padding: 8px; }
  }
</style>
