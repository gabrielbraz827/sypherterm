<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import type { TerminalPaneNode } from '../layout';
  import type { TerminalConnection } from '../websocket';
  import type { UserPreferences } from '../api';
  import SessionStatus from './SessionStatus.svelte';
  import TerminalInstance from './TerminalInstance.svelte';
  import TerminalToolbar from './TerminalToolbar.svelte';

  export let pane: TerminalPaneNode;
  export let connection: TerminalConnection | undefined = undefined;
  export let preferences: UserPreferences | null = null;
  export let active = false;
  export let state = 'idle';
  export let message = '';

  const dispatch = createEventDispatcher<{
    focus: { paneId: string };
    split: { paneId: string; direction: 'horizontal' | 'vertical' };
    close: { paneId: string };
    reconnect: { paneId: string };
    disconnected: { paneId: string };
    status: { paneId: string; state: string; message?: string };
  }>();

  let terminalRef: {
    copySelection: () => Promise<void>;
    pasteClipboard: () => Promise<void>;
    clearTerminal: () => void;
    insertText: (text: string) => void;
    disconnect: () => Promise<void>;
  } | null = null;

  export function insertText(text: string) {
    terminalRef?.insertText(text);
  }

  async function disconnectPane() {
    await terminalRef?.disconnect();
    dispatch('disconnected', { paneId: pane.paneId });
  }

</script>

<section
  class={`flex min-h-[280px] min-w-0 flex-col overflow-hidden rounded-panel border bg-black font-mono ${
    active ? 'border-sypher-accent' : 'border-sypher-border'
  }`}
>
  <div class="flex items-center justify-between gap-2 border-b border-sypher-border px-3 py-2">
    <div class="min-w-0">
      <p class="truncate text-xs font-medium text-sypher-text">{pane.title ?? 'Terminal'}</p>
      <p class="truncate text-[11px] text-sypher-muted">{pane.sessionId ?? 'No session'}</p>
    </div>
    <div class="flex shrink-0 items-center gap-2">
      <button
        class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
        type="button"
        on:click={() => dispatch('focus', { paneId: pane.paneId })}
      >
        Focus
      </button>
      <button
        class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
        type="button"
        on:click|stopPropagation={() => dispatch('split', { paneId: pane.paneId, direction: 'horizontal' })}
      >
        Split H
      </button>
      <button
        class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
        type="button"
        on:click|stopPropagation={() => dispatch('split', { paneId: pane.paneId, direction: 'vertical' })}
      >
        Split V
      </button>
      <button
        class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
        type="button"
        on:click|stopPropagation={() => dispatch('close', { paneId: pane.paneId })}
      >
        Close
      </button>
    </div>
  </div>

  <TerminalToolbar
    connected={state === 'connected'}
    busy={state === 'connecting'}
    on:copy={() => terminalRef?.copySelection()}
    on:paste={() => terminalRef?.pasteClipboard()}
    on:clear={() => terminalRef?.clearTerminal()}
    on:reconnect={() => dispatch('reconnect', { paneId: pane.paneId })}
    on:disconnect={disconnectPane}
  />

  <TerminalInstance
    bind:this={terminalRef}
    {connection}
    {preferences}
    on:status={(event) =>
      dispatch('status', {
        paneId: pane.paneId,
        state: event.detail.state,
        message: event.detail.message,
      })}
  />

  <div class="border-t border-sypher-border p-2">
    <SessionStatus {state} {message} />
  </div>
</section>
