<script lang="ts">
  export let state = 'idle';
  export let message = '';

  const labels: Record<string, string> = {
    idle: 'Idle',
    connecting: 'Connecting',
    connected: 'Connected',
    closed: 'Closed',
    failed: 'Failed',
  };

  $: tone =
    state === 'connected'
      ? 'border-emerald-900/60 bg-emerald-950/40 text-emerald-200'
      : state === 'failed'
        ? 'border-red-900/60 bg-red-950/40 text-red-200'
        : 'border-sypher-border bg-sypher-surface text-sypher-muted';
</script>

<div class={`rounded-panel border px-3 py-2 text-xs ${tone}`}>
  <div class="flex items-center justify-between gap-3">
    <span>{labels[state] ?? state}</span>
    {#if message}
      <span class="truncate text-right">{message}</span>
    {/if}
  </div>
</div>
