<script lang="ts">
  import type { ImportSummary, JobStatus } from '../types';

  interface Props {
    status?: JobStatus;
    summary?: ImportSummary;
    error?: string;
  }

  let { status, summary, error }: Props = $props();
  const progress = $derived(status?.total ? Math.min(100, Math.round((status.processed / status.total) * 100)) : 0);
  const active = $derived(Boolean(status?.total && status.processed < status.total));
</script>

<div class="import-status" aria-live="polite" aria-atomic="true">
  <div class="status-copy">
    <span class:active class="pulse" aria-hidden="true"></span>
    {#if error}
      <strong>Import error</strong><span>{error}</span>
    {:else if active && status}
      <strong>{status.phase || 'Working'}</strong>
      <span>{status.processed.toLocaleString()} of {status.total.toLocaleString()}</span>
      {#if status.failed}<span>· {status.failed.toLocaleString()} failed</span>{/if}
      {#if status.message}<span class="message">· {status.message}</span>{/if}
    {:else if summary}
      <strong>Import complete</strong>
      <span>{summary.imported.toLocaleString()} new · {summary.existing.toLocaleString()} existing · {summary.pending.toLocaleString()} pending · {summary.failed.toLocaleString()} failed</span>
    {:else if status?.message}
      <strong>{status.phase || 'Archive ready'}</strong><span>{status.message}</span>
    {:else}
      <strong>Local archive</strong><span>Ready</span>
    {/if}
  </div>
  {#if active}
    <div class="progress" role="progressbar" aria-label="Import progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={progress}>
      <span style={`width: ${progress}%`}></span>
    </div>
  {/if}
</div>
