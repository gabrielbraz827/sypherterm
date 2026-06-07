<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let connected = false;
  export let busy = false;

  const dispatch = createEventDispatcher<{
    copy: void;
    paste: void;
    clear: void;
    reconnect: void;
    disconnect: void;
  }>();
</script>

<div class="flex h-11 items-center justify-between gap-3 border-b border-sypher-border px-3">
  <div class="flex min-w-0 items-center gap-2">
    <button
      class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text disabled:opacity-40"
      type="button"
      disabled={busy}
      on:click={() => dispatch('copy')}
    >
      Copy
    </button>
    <button
      class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text disabled:opacity-40"
      type="button"
      disabled={!connected || busy}
      on:click={() => dispatch('paste')}
    >
      Paste
    </button>
    <button
      class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text disabled:opacity-40"
      type="button"
      disabled={busy}
      on:click={() => dispatch('clear')}
    >
      Clear
    </button>
  </div>

  <div class="flex items-center gap-2">
    <button
      class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text disabled:opacity-40"
      type="button"
      disabled={busy}
      on:click={() => dispatch('reconnect')}
    >
      Reconnect
    </button>
    <button
      class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text disabled:opacity-40"
      type="button"
      disabled={!connected || busy}
      on:click={() => dispatch('disconnect')}
    >
      Disconnect
    </button>
  </div>
</div>
