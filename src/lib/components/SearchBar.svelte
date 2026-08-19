<script lang="ts">
  interface Props {
    value: string;
    busy?: boolean;
    oninput: (value: string) => void;
  }

  let { value, busy = false, oninput }: Props = $props();
</script>

<div class="search-wrap" role="search">
  <svg aria-hidden="true" viewBox="0 0 24 24"><path d="m21 21-4.4-4.4m2.4-5.1a7.5 7.5 0 1 1-15 0 7.5 7.5 0 0 1 15 0Z" /></svg>
  <input
    type="search"
    aria-label="Search archive"
    placeholder="Search posts…"
    autocomplete="off"
    {value}
    oninput={(event) => oninput(event.currentTarget.value)}
  />
  {#if busy}<span class="searching" aria-label="Searching"></span>{/if}
</div>
