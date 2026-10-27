<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import FilterBar from './lib/components/FilterBar.svelte';
  import ImportStatus from './lib/components/ImportStatus.svelte';
  import PlaylistNav from './lib/components/PlaylistNav.svelte';
  import ProfileSwitcher from './lib/components/ProfileSwitcher.svelte';
  import SearchBar from './lib/components/SearchBar.svelte';
  import TweetCard from './lib/components/TweetCard.svelte';
  import {
    createPlaylist, deletePlaylist, getImportStatus, getPlaylists, getProfiles, getTweet, getTweets, importArchive,
    openOriginal, retryEnrichment, setActiveProfile, setPlaylistMembership, updatePlaylist
  } from './lib/api';
  import type { Filters, ImportSummary, JobStatus, Playlist, ProfileState, SortOrder, Tweet, TweetQuery } from './lib/types';

  const PAGE_SIZE = 40;
  const emptyFilters: Filters = { author: '', mediaType: '', after: '', before: '', status: '' };

  let searchInput = $state('');
  let search = $state('');
  let filters = $state<Filters>({ ...emptyFilters });
  let sortOrder = $state<SortOrder>('newest');
  let tweets = $state<Tweet[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let importing = $state(false);
  let error = $state<string>();
  let actionError = $state<string>();
  let importError = $state<string>();
  let importSummary = $state<ImportSummary>();
  let jobStatus = $state<JobStatus>();
  let profileState = $state<ProfileState>({ profiles: [] });
  let profilesLoading = $state(true);
  let profileError = $state<string>();
  let busyProfileId = $state<string>();
  let expandedId = $state<string>();
  let detailBusyId = $state<string>();
  let actionBusyId = $state<string>();
  let playlists = $state<Playlist[]>([]);
  let activePlaylistId = $state<number>();
  let playlistError = $state<string>();
  let editingPlaylist = $state(false);
  let playlistDraft = $state({ name: '', description: '' });
  let confirmingDelete = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let requestSequence = 0;
  let refreshingTweets = false;

  const hasMore = $derived(tweets.length < total);
  const activePlaylist = $derived(playlists.find((playlist) => playlist.id === activePlaylistId));
  const likeCount = $derived(profileState.profiles.find((profile) => profile.id === profileState.activeProfileId)?.postCount ?? 0);
  const activeFilterCount = $derived(
    Number(Boolean(filters.author)) + Number(Boolean(filters.mediaType)) + Number(Boolean(filters.after)) +
    Number(Boolean(filters.before)) + Number(Boolean(filters.status))
  );

  function message(value: unknown): string {
    if (value instanceof Error) return value.message;
    return typeof value === 'string' ? value : 'Something went wrong.';
  }

  function buildQuery(offset: number, limit = PAGE_SIZE): TweetQuery {
    return {
      playlistId: activePlaylistId,
      search: search || undefined,
      author: filters.author.trim() || undefined,
      mediaType: filters.mediaType || undefined,
      before: filters.before || undefined,
      after: filters.after || undefined,
      status: filters.status || undefined,
      sortOrder,
      offset,
      limit
    };
  }

  async function loadTweets(append = false): Promise<void> {
    const sequence = ++requestSequence;
    append ? loadingMore = true : loading = true;
    if (!append) error = undefined;
    try {
      const page = await getTweets(buildQuery(append ? tweets.length : 0));
      if (sequence !== requestSequence) return;
      tweets = append ? [...tweets, ...page.items] : page.items;
      total = page.total;
    } catch (cause) {
      if (sequence === requestSequence) error = message(cause);
    } finally {
      if (sequence === requestSequence) {
        loading = false;
        loadingMore = false;
      }
    }
  }

  async function refreshVisibleTweets(): Promise<void> {
    if (refreshingTweets || loading || loadingMore || tweets.length === 0) return;
    refreshingTweets = true;
    const sequence = requestSequence;
    const target = tweets.length;
    const refreshed: Tweet[] = [];
    let refreshedTotal = total;

    try {
      for (let offset = 0; offset < target; offset += 100) {
        const page = await getTweets(buildQuery(offset, Math.min(100, target - offset)));
        refreshed.push(...page.items);
        refreshedTotal = page.total;
        if (page.items.length === 0 || refreshed.length >= page.total) break;
      }
      if (sequence === requestSequence) {
        tweets = refreshed;
        total = refreshedTotal;
      }
    } catch {
      // Background refresh failures should not replace a usable feed with an error state.
    } finally {
      refreshingTweets = false;
    }
  }

  function scheduleSearch(value: string): void {
    searchInput = value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      search = value.trim();
      expandedId = undefined;
      void loadTweets();
    }, 280);
  }

  function changeFilters(next: Filters): void {
    filters = next;
    expandedId = undefined;
    void loadTweets();
  }

  function clearFilters(): void {
    filters = { ...emptyFilters };
    void loadTweets();
  }

  function toggleSortOrder(): void {
    sortOrder = sortOrder === 'newest' ? 'oldest' : 'newest';
    expandedId = undefined;
    void loadTweets();
  }

  async function chooseArchive(directory: boolean): Promise<void> {
    actionError = undefined;
    importError = undefined;
    const selected = await open({
      directory,
      multiple: false,
      title: directory ? 'Choose extracted X archive folder' : 'Choose X archive ZIP',
      filters: directory ? undefined : [{ name: 'ZIP archive', extensions: ['zip'] }]
    });
    if (typeof selected !== 'string') return;
    importing = true;
    importSummary = undefined;
    try {
      const importPromise = importArchive(selected);
      void refreshStatus();
      importSummary = await importPromise;
      await Promise.all([loadTweets(), refreshProfiles(), refreshPlaylists(), refreshStatus()]);
    } catch (cause) {
      importError = message(cause);
    } finally {
      importing = false;
    }
  }
  async function refreshProfiles(): Promise<void> {
    profilesLoading = true;
    profileError = undefined;
    try {
      profileState = await getProfiles();
    } catch (cause) {
      profileError = message(cause);
    } finally {
      profilesLoading = false;
    }
  }

  async function switchProfile(profileId: string): Promise<void> {
    if (busyProfileId || profileId === profileState.activeProfileId) return;
    busyProfileId = profileId;
    actionError = undefined;
    try {
      await setActiveProfile(profileId);
      profileState = { ...profileState, activeProfileId: profileId };
      activePlaylistId = undefined;
      editingPlaylist = false;
      confirmingDelete = false;
      expandedId = undefined;
      detailBusyId = undefined;
      tweets = [];
      total = 0;
      await Promise.all([refreshProfiles(), refreshPlaylists(), loadTweets()]);
    } catch (cause) {
      actionError = `Could not switch accounts: ${message(cause)}`;
    } finally {
      busyProfileId = undefined;
    }
  }


  async function refreshPlaylists(): Promise<void> {
    try {
      playlists = await getPlaylists();
      playlistError = undefined;
      if (activePlaylistId !== undefined && !playlists.some((playlist) => playlist.id === activePlaylistId)) {
        selectPlaylist(undefined);
      }
    } catch (cause) {
      playlistError = message(cause);
    }
  }

  // Agents edit playlists straight in the database, so pick up their changes when the window regains focus.
  async function syncPlaylistsOnFocus(): Promise<void> {
    const before = activePlaylist?.updatedAt;
    await refreshPlaylists();
    if (activePlaylist && activePlaylist.updatedAt !== before) void loadTweets();
  }

  function replacePlaylist(updated: Playlist): void {
    const exists = playlists.some((playlist) => playlist.id === updated.id);
    const next = exists ? playlists.map((playlist) => playlist.id === updated.id ? updated : playlist) : [...playlists, updated];
    playlists = next.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
  }

  function selectPlaylist(playlistId?: number): void {
    if (playlistId === activePlaylistId) return;
    activePlaylistId = playlistId;
    editingPlaylist = false;
    confirmingDelete = false;
    expandedId = undefined;
    window.scrollTo({ top: 0 });
    void loadTweets();
  }

  async function createPlaylistFromNav(name: string): Promise<boolean> {
    playlistError = undefined;
    try {
      replacePlaylist(await createPlaylist(name));
      return true;
    } catch (cause) {
      playlistError = message(cause);
      return false;
    }
  }

  async function togglePlaylist(tweetId: string, playlistId: number, included: boolean): Promise<void> {
    actionError = undefined;
    try {
      replacePlaylist(await setPlaylistMembership(playlistId, tweetId, included));
      if (!included && playlistId === activePlaylistId) {
        tweets = tweets.filter((tweet) => tweet.id !== tweetId);
        total = Math.max(0, total - 1);
        return;
      }
      tweets = tweets.map((tweet) => tweet.id === tweetId
        ? { ...tweet, playlistIds: included ? [...tweet.playlistIds, playlistId] : tweet.playlistIds.filter((id) => id !== playlistId) }
        : tweet);
    } catch (cause) {
      actionError = `Could not update playlist: ${message(cause)}`;
    }
  }

  async function createPlaylistWithTweet(tweetId: string, name: string): Promise<boolean> {
    actionError = undefined;
    try {
      const created = await createPlaylist(name);
      replacePlaylist(created);
      await togglePlaylist(tweetId, created.id, true);
      return true;
    } catch (cause) {
      actionError = `Could not create playlist: ${message(cause)}`;
      return false;
    }
  }

  function startEditingPlaylist(): void {
    if (!activePlaylist) return;
    playlistDraft = { name: activePlaylist.name, description: activePlaylist.description };
    confirmingDelete = false;
    editingPlaylist = true;
  }

  async function savePlaylist(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!activePlaylist) return;
    actionError = undefined;
    try {
      replacePlaylist(await updatePlaylist(activePlaylist.id, playlistDraft.name, playlistDraft.description));
      editingPlaylist = false;
    } catch (cause) {
      actionError = `Could not save playlist: ${message(cause)}`;
    }
  }

  async function removePlaylist(): Promise<void> {
    if (!activePlaylist) return;
    actionError = undefined;
    try {
      const removedId = activePlaylist.id;
      await deletePlaylist(removedId);
      playlists = playlists.filter((playlist) => playlist.id !== removedId);
      tweets = tweets.map((tweet) => ({ ...tweet, playlistIds: tweet.playlistIds.filter((id) => id !== removedId) }));
      selectPlaylist(undefined);
    } catch (cause) {
      actionError = `Could not delete playlist: ${message(cause)}`;
    }
  }

  async function refreshStatus(): Promise<void> {
    try {
      const previous = jobStatus;
      const next = await getImportStatus();
      jobStatus = next;
      if (next.processed !== previous?.processed || next.phase !== previous?.phase) {
        void refreshVisibleTweets();
      }
    } catch {
      // The feed error provides actionable startup feedback; status polling stays quiet.
    }
  }

  async function toggleDetails(id: string): Promise<void> {
    if (expandedId === id) {
      expandedId = undefined;
      return;
    }
    detailBusyId = id;
    actionError = undefined;
    try {
      const detail = await getTweet(id);
      tweets = tweets.map((tweet) => tweet.id === id ? detail : tweet);
      expandedId = id;
    } catch (cause) {
      actionError = `Could not load post details: ${message(cause)}`;
    } finally {
      detailBusyId = undefined;
    }
  }

  async function retry(id: string): Promise<void> {
    actionBusyId = id;
    actionError = undefined;
    try {
      await retryEnrichment(id);
      const detail = await getTweet(id);
      tweets = tweets.map((tweet) => tweet.id === id ? detail : tweet);
      await refreshStatus();
    } catch (cause) {
      actionError = `Could not retry enrichment: ${message(cause)}`;
    } finally {
      actionBusyId = undefined;
    }
  }

  async function openPost(id: string): Promise<void> {
    actionBusyId = id;
    actionError = undefined;
    try {
      await openOriginal(id);
    } catch (cause) {
      actionError = `Could not open the original post: ${message(cause)}`;
    } finally {
      actionBusyId = undefined;
    }
  }

  onMount(() => {
    void Promise.all([loadTweets(), refreshProfiles(), refreshPlaylists(), refreshStatus()]);
    const statusTimer = setInterval(() => void refreshStatus(), 1500);
    const handleFocus = () => void syncPlaylistsOnFocus();
    window.addEventListener('focus', handleFocus);
    return () => {
      clearInterval(statusTimer);
      window.removeEventListener('focus', handleFocus);
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });
</script>

<svelte:head><title>X Archive — Local likes browser</title></svelte:head>

<div class="app-shell">
  <header class="topbar">
    <div class="brand" aria-label="X Archive home">
      <span class="brand-mark" aria-hidden="true">X</span>
      <span><strong>Archive</strong><small>Local likes browser</small></span>
    </div>
    <SearchBar value={searchInput} busy={loading && Boolean(search)} oninput={scheduleSearch} />
    <div class="import-actions">
      <button class="secondary-button" type="button" onclick={() => void chooseArchive(true)} disabled={importing}>Import folder</button>
      <button class="primary-button" type="button" onclick={() => void chooseArchive(false)} disabled={importing}>
        {importing ? 'Importing…' : 'Import ZIP'}
      </button>
    </div>
  </header>

  <main>
    <aside class="sidebar">
      <section>
        <div class="section-heading"><h2>Filters</h2>{#if activeFilterCount}<span>{activeFilterCount}</span>{/if}</div>
        <FilterBar {filters} onchange={changeFilters} onclear={clearFilters} />
      </section>
      <PlaylistNav
        {playlists}
        {activePlaylistId}
        {likeCount}
        error={playlistError}
        onselect={selectPlaylist}
        oncreate={createPlaylistFromNav}
      />
      <section class="library-summary" aria-label="Library summary">
        <h2>Library</h2>
        <div><span>Matching posts</span><strong>{total.toLocaleString()}</strong></div>
        <div><span>Loaded now</span><strong>{tweets.length.toLocaleString()}</strong></div>
        <p>Everything stays on this device.</p>
      </section>
      <ProfileSwitcher
        profiles={profileState.profiles}
        activeProfileId={profileState.activeProfileId}
        loading={profilesLoading}
        {busyProfileId}
        error={profileError}
        onselect={(profileId) => void switchProfile(profileId)}
      />
    </aside>

    <section class="feed" aria-labelledby="feed-title" aria-busy={loading}>
      <div class="feed-heading">
        <div>
          <h1 id="feed-title">
            {#if activePlaylist}{search ? `“${search}” in ${activePlaylist.name}` : activePlaylist.name}
            {:else}{search ? `Results for “${search}”` : 'Liked posts'}{/if}
          </h1>
          {#if activePlaylist?.description}<p class="playlist-description">{activePlaylist.description}</p>{/if}
          <p>{loading ? 'Searching local archive…' : `${total.toLocaleString()} ${total === 1 ? 'post' : 'posts'}`}</p>
        </div>
        {#if activePlaylist}
          <div class="playlist-actions">
            {#if confirmingDelete}
              <span>Delete “{activePlaylist.name}”? Posts stay in your likes.</span>
              <button class="sort-button danger" type="button" onclick={() => void removePlaylist()}>Delete</button>
              <button class="sort-button" type="button" onclick={() => confirmingDelete = false}>Cancel</button>
            {:else}
              <button class="sort-button" type="button" onclick={startEditingPlaylist}>Edit</button>
              <button class="sort-button" type="button" onclick={() => { confirmingDelete = true; editingPlaylist = false; }}>Delete</button>
            {/if}
          </div>
        {:else}
        <button
          class="sort-button"
          type="button"
          aria-label={sortOrder === 'newest' ? 'Show oldest posts first' : 'Show newest posts first'}
          title="Sort by post chronology"
          onclick={toggleSortOrder}
        >
          <svg aria-hidden="true" viewBox="0 0 24 24"><path d="M8 7h10M8 12h7M8 17h4M4 5v14m0 0-2.5-2.5M4 19l2.5-2.5" /></svg>
          {sortOrder === 'newest' ? 'Newest first' : 'Oldest first'}
        </button>
        {/if}
      </div>

      {#if editingPlaylist && activePlaylist}
        <form class="playlist-editor" onsubmit={(event) => void savePlaylist(event)}>
          <label><span>Name</span><input bind:value={playlistDraft.name} maxlength="80" required /></label>
          <label><span>Description</span><textarea bind:value={playlistDraft.description} rows="2" maxlength="500"></textarea></label>
          <div>
            <button class="primary-button" type="submit" disabled={!playlistDraft.name.trim()}>Save</button>
            <button class="secondary-button" type="button" onclick={() => editingPlaylist = false}>Cancel</button>
          </div>
        </form>
      {/if}

      {#if actionError}
        <div class="inline-error" role="alert"><span>{actionError}</span><button type="button" onclick={() => actionError = undefined}>Dismiss</button></div>
      {/if}

      {#if loading}
        <div class="skeleton-list" aria-label="Loading posts">
          {#each Array(4) as _}<div class="skeleton-card"><span></span><div><i></i><i></i><i></i></div></div>{/each}
        </div>
      {:else if error}
        <div class="state-panel error-state" role="alert">
          <strong>Couldn’t load the archive</strong><p>{error}</p><button class="primary-button" type="button" onclick={() => void loadTweets()}>Try again</button>
        </div>
      {:else if tweets.length === 0 && activePlaylist && !search && !activeFilterCount}
        <div class="state-panel">
          <strong>This playlist is empty</strong>
          <p>Use “Add to playlist” on any post in All likes, or ask an agent to fill it from a search.</p>
          <button class="secondary-button" type="button" onclick={() => selectPlaylist(undefined)}>Browse all likes</button>
        </div>
      {:else if tweets.length === 0}
        <div class="state-panel">
          <strong>{search || activeFilterCount ? 'No posts match this view' : 'Import your X archive to begin'}</strong>
          <p>{search || activeFilterCount ? 'Try a broader search or clear one of the filters.' : 'Choose the original ZIP or its extracted folder. Importing again is safe.'}</p>
          {#if search || activeFilterCount}
            <button class="secondary-button" type="button" onclick={() => { searchInput = ''; search = ''; filters = { ...emptyFilters }; void loadTweets(); }}>Clear search and filters</button>
          {:else}
            <button class="primary-button" type="button" onclick={() => void chooseArchive(false)}>Choose archive ZIP</button>
          {/if}
        </div>
      {:else}
        <div class="tweet-list">
          {#each tweets as tweet (tweet.id)}
            <TweetCard
              {tweet}
              {playlists}
              onplaylisttoggle={togglePlaylist}
              onplaylistcreate={createPlaylistWithTweet}
              expanded={expandedId === tweet.id}
              detailBusy={detailBusyId === tweet.id}
              actionBusy={actionBusyId === tweet.id}
              ondetail={toggleDetails}
              onretry={retry}
              onopen={openPost}
            />
          {/each}
        </div>
        {#if hasMore}
          <div class="load-more">
            <button class="secondary-button" type="button" onclick={() => void loadTweets(true)} disabled={loadingMore}>
              {loadingMore ? 'Loading…' : `Load more · ${Math.max(0, total - tweets.length).toLocaleString()} remaining`}
            </button>
          </div>
        {/if}
      {/if}
    </section>
  </main>

  <ImportStatus status={jobStatus} summary={importSummary} error={importError} />
</div>
