<script lang="ts">
  import { tick } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { Profile } from '../types';

  interface Props {
    profiles: Profile[];
    activeProfileId?: string;
    loading?: boolean;
    busyProfileId?: string;
    error?: string;
    onselect: (profileId: string) => void;
  }

  let { profiles, activeProfileId, loading = false, busyProfileId, error, onselect }: Props = $props();
  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();

  const activeProfile = $derived(profiles.find((profile) => profile.id === activeProfileId));

  function avatarSource(profile: Profile): string | undefined {
    return profile.avatarLocalPath ? convertFileSrc(profile.avatarLocalPath) : profile.avatarUrl;
  }

  function fallbackAvatar(event: Event, profile: Profile): void {
    const image = event.currentTarget as HTMLImageElement;
    if (image.dataset.local === 'true' && profile.avatarUrl) {
      image.dataset.local = 'false';
      image.src = profile.avatarUrl;
    } else {
      image.hidden = true;
    }
  }

  async function showMenu(): Promise<void> {
    if (!profiles.length) return;
    open = true;
    await tick();
    const current = menu?.querySelector<HTMLButtonElement>('[aria-checked="true"]');
    (current ?? menu?.querySelector<HTMLButtonElement>('[role="menuitemradio"]'))?.focus();
  }

  function closeMenu(restoreFocus = false): void {
    open = false;
    if (restoreFocus) trigger?.focus();
  }

  function toggleMenu(): void {
    if (open) closeMenu();
    else void showMenu();
  }

  function selectProfile(profile: Profile): void {
    closeMenu(true);
    if (profile.id !== activeProfileId && !busyProfileId) onselect(profile.id);
  }

  function handleWindowClick(event: MouseEvent): void {
    if (open && !menu?.contains(event.target as Node) && !trigger?.contains(event.target as Node)) closeMenu();
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      closeMenu(true);
    }
  }

  function handleTriggerKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      event.preventDefault();
      void showMenu();
    }
  }

  function handleMenuKeydown(event: KeyboardEvent): void {
    if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const items = Array.from(menu?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]:not(:disabled)') ?? []);
    if (!items.length) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'Home') items[0]?.focus();
    else if (event.key === 'End') items.at(-1)?.focus();
    else {
      const direction = event.key === 'ArrowDown' ? 1 : -1;
      items[(current + direction + items.length) % items.length]?.focus();
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="profile-switcher">
  {#if open}
    <div class="profile-menu" bind:this={menu} role="menu" tabindex="-1" aria-label="Archive accounts" onkeydown={handleMenuKeydown}>
      <div class="profile-menu-heading">Accounts</div>
      {#each profiles as profile (profile.id)}
        <button
          class="profile-option"
          type="button"
          role="menuitemradio"
          aria-checked={profile.id === activeProfileId}
          disabled={Boolean(busyProfileId)}
          onclick={() => selectProfile(profile)}
        >
          <span class="profile-avatar" aria-hidden="true">
            <span>{profile.displayName.slice(0, 1).toUpperCase() || '?'}</span>
            {#if avatarSource(profile)}
              <img
                src={avatarSource(profile)}
                alt=""
                data-local={profile.avatarLocalPath ? 'true' : 'false'}
                onerror={(event) => fallbackAvatar(event, profile)}
              />
            {/if}
          </span>
          <span class="profile-identity">
            <strong>{profile.displayName}</strong>
            <span>{profile.username ? `@${profile.username}` : 'Archive profile'}</span>
            <small>{profile.postCount.toLocaleString()} {profile.postCount === 1 ? 'post' : 'posts'}</small>
          </span>
          {#if profile.id === activeProfileId}
            <svg class="profile-check" aria-label="Current account" viewBox="0 0 24 24"><path d="m5 12.5 4.2 4.2L19 7" /></svg>
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  <button
    class="profile-trigger"
    type="button"
    bind:this={trigger}
    aria-haspopup="menu"
    aria-expanded={open}
    disabled={loading || profiles.length === 0}
    onclick={toggleMenu}
    onkeydown={handleTriggerKeydown}
  >
    {#if activeProfile}
      <span class="profile-avatar" aria-hidden="true">
        <span>{activeProfile.displayName.slice(0, 1).toUpperCase() || '?'}</span>
        {#if avatarSource(activeProfile)}
          <img
            src={avatarSource(activeProfile)}
            alt=""
            data-local={activeProfile.avatarLocalPath ? 'true' : 'false'}
            onerror={(event) => fallbackAvatar(event, activeProfile)}
          />
        {/if}
      </span>
      <span class="profile-identity">
        <strong>{activeProfile.displayName}</strong>
        <span>{activeProfile.username ? `@${activeProfile.username}` : 'Archive profile'}</span>
      </span>
      <svg class="profile-chevron" aria-hidden="true" viewBox="0 0 24 24"><path d="m7 14 5-5 5 5" /></svg>
    {:else}
      <span class="profile-placeholder">
        <strong>{loading ? 'Loading accounts…' : error ? 'Accounts unavailable' : 'No archive profile'}</strong>
        <span>{error ?? (loading ? 'Checking local archive' : 'Import an archive to add one')}</span>
      </span>
    {/if}
  </button>
</div>
