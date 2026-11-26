<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../../mobile/src/Icon.svelte';
  import { createPlaylist, setPlaylistMembership, updatePlaylist } from '../api';
  import { desk, message } from '../state.svelte';

  // Create a list (optionally saving a post into it right away) or edit one.
  const editing = desk.listDialog?.playlist;
  const addTweet = desk.listDialog?.addTweet;
  let name = $state(editing?.name ?? '');
  let description = $state(editing?.description ?? '');
  let saving = $state(false);
  let problem = $state<string>();
  let field = $state<HTMLInputElement>();

  const close = () => (desk.listDialog = undefined);

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!name.trim() || saving) return;
    saving = true;
    problem = undefined;
    try {
      if (editing) {
        desk.replacePlaylist(await updatePlaylist(editing.id, name.trim(), description.trim()));
      } else {
        let created = await createPlaylist(name.trim(), description.trim());
        if (addTweet) created = await setPlaylistMembership(created.id, addTweet, true);
        desk.replacePlaylist(created);
        desk.showToast(addTweet ? `Saved to ${created.name}` : `Made “${created.name}”`);
        if (addTweet) desk.markListed(addTweet, created.id);
      }
      close();
    } catch (cause) {
      problem = message(cause);
    } finally {
      saving = false;
    }
  }

  onMount(() => {
    field?.focus();
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
    window.addEventListener('keydown', key, true);
    return () => window.removeEventListener('keydown', key, true);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="dialog-scrim" onclick={(e) => { if (e.target === e.currentTarget) close(); }}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="list-title">
  <form onsubmit={(e) => void save(e)}>
    <header>
      <button type="button" class="round" aria-label="Close" onclick={close}><Icon name="close" size={20} /></button>
      <h2 id="list-title">{editing ? 'Edit list' : 'New list'}</h2>
      <button type="submit" class="pill primary small" disabled={!name.trim() || saving}>{saving ? 'Saving…' : editing ? 'Save' : 'Create'}</button>
    </header>
    <div class="body">
      <label>
        <span>Name <em>{name.length} / 80</em></span>
        <input class="field" bind:this={field} bind:value={name} maxlength="80" required />
      </label>
      <label>
        <span>Description <em>{description.length} / 500</em></span>
        <textarea class="field" bind:value={description} rows="3" maxlength="500"></textarea>
      </label>
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    </div>
  </form>
  </div>
</div>

<style>
  form > header {
    position: sticky;
    top: 0;
    display: flex;
    align-items: center;
    gap: 24px;
    height: 53px;
    padding: 0 16px;
    background: var(--bg-blur);
    backdrop-filter: blur(12px);
  }
  form > header h2 { font-size: 20px; font-weight: 700; margin: 0; flex: 1; }
  .body { padding: 8px 16px 24px; display: grid; gap: 20px; }
  label { display: grid; gap: 6px; }
  label span { display: flex; justify-content: space-between; color: var(--muted); font-size: 13px; }
  em { font-style: normal; }
  textarea { resize: vertical; }
  .problem { margin: 0; padding: 12px; border-radius: 12px; color: var(--danger); background: var(--danger-soft); }
</style>
