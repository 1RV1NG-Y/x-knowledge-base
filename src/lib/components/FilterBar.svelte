<script lang="ts">
  import type { Filters } from '../types';

  interface Props {
    filters: Filters;
    onchange: (filters: Filters) => void;
    onclear: () => void;
  }

  let { filters, onchange, onclear }: Props = $props();

  function update<K extends keyof Filters>(key: K, value: Filters[K]): void {
    onchange({ ...filters, [key]: value });
  }

  function commitDate(key: 'after' | 'before', input: HTMLInputElement): void {
    const compact = input.value.trim();
    const value = /^\d{8}$/.test(compact)
      ? compact.replace(/^(\d{4})(\d{2})(\d{2})$/, '$1-$2-$3')
      : compact;
    const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    const date = match
      ? new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])))
      : undefined;
    const valid = value === '' || Boolean(
      date &&
      date.getUTCFullYear() === Number(match![1]) &&
      date.getUTCMonth() === Number(match![2]) - 1 &&
      date.getUTCDate() === Number(match![3])
    );

    if (!valid) {
      input.setCustomValidity('Enter a valid date as YYYY-MM-DD.');
      input.reportValidity();
      return;
    }

    input.setCustomValidity('');
    input.value = value;
    update(key, value);
  }

  function commitDateOnEnter(event: KeyboardEvent): void {
    if (event.key === 'Enter') (event.currentTarget as HTMLInputElement).blur();
  }
</script>

<div class="filters" aria-label="Archive filters">
  <label>
    <span>Author</span>
    <input
      type="text"
      value={filters.author}
      placeholder="username"
      autocomplete="off"
      oninput={(event) => update('author', event.currentTarget.value)}
    />
  </label>
  <label>
    <span>Media</span>
    <select value={filters.mediaType} onchange={(event) => update('mediaType', event.currentTarget.value as Filters['mediaType'])}>
      <option value="">Any</option>
      <option value="image">Images</option>
      <option value="video">Videos</option>
    </select>
  </label>
  <label>
    <span>After</span>
    <input
      type="text"
      value={filters.after}
      placeholder="YYYY-MM-DD"
      inputmode="numeric"
      maxlength="10"
      autocomplete="off"
      spellcheck="false"
      onchange={(event) => commitDate('after', event.currentTarget)}
      onkeydown={commitDateOnEnter}
    />
  </label>
  <label>
    <span>Before</span>
    <input
      type="text"
      value={filters.before}
      placeholder="YYYY-MM-DD"
      inputmode="numeric"
      maxlength="10"
      autocomplete="off"
      spellcheck="false"
      onchange={(event) => commitDate('before', event.currentTarget)}
      onkeydown={commitDateOnEnter}
    />
  </label>
  <label>
    <span>Status</span>
    <select value={filters.status} onchange={(event) => update('status', event.currentTarget.value)}>
      <option value="">Any</option>
      <option value="ok">Enriched</option>
      <option value="pending">Pending</option>
      <option value="failed">Failed</option>
      <option value="unavailable">Unavailable</option>
    </select>
  </label>
  <button class="clear-button" type="button" onclick={onclear} disabled={!filters.author && !filters.mediaType && !filters.after && !filters.before && !filters.status}>Clear</button>
</div>
