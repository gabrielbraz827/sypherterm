<script lang="ts">
  import { FitAddon } from '@xterm/addon-fit';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { Terminal } from '@xterm/xterm';
  import '@xterm/xterm/css/xterm.css';
  import { createEventDispatcher, onMount } from 'svelte';

  import { disconnectSession, resizeSession, type UserPreferences } from '../api';
  import { copyTerminalSelection, readTerminalClipboard } from '../native';
  import {
    TerminalSocket,
    type TerminalConnection,
    type TerminalSocketState,
  } from '../websocket';

  export let connection: TerminalConnection | null = null;
  export let preferences: UserPreferences | null = null;

  const dispatch = createEventDispatcher<{
    status: { state: TerminalSocketState | 'idle'; message?: string };
  }>();

  let host: HTMLDivElement;
  let terminal: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let socket: TerminalSocket | null = null;
  let observer: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let currentSessionId = '';
  let lastResize = '';
  let state: TerminalSocketState | 'idle' = 'idle';

  const decoder = new TextDecoder();

  onMount(() => {
    mountTerminal();
    observer = new ResizeObserver(scheduleFit);
    observer.observe(host);

    if (connection) {
      void connectToSession(connection);
    }

    return () => {
      cleanup(false);
      observer?.disconnect();
      terminal?.dispose();
    };
  });

  $: if (terminal && preferences) {
    applyPreferences();
  }

  $: if (terminal && connection && connection.sessionId !== currentSessionId) {
    void connectToSession(connection);
  }

  export async function copySelection() {
    const selection = terminal?.getSelection();
    if (!selection) {
      return;
    }

    await copyTerminalSelection(selection);
  }

  export async function pasteClipboard() {
    const text = await readTerminalClipboard();
    if (!text) {
      return;
    }

    socket?.sendInput(text);
    terminal?.focus();
  }

  export function clearTerminal() {
    terminal?.clear();
    terminal?.focus();
  }

  export function insertText(text: string) {
    socket?.sendInput(text);
    terminal?.focus();
  }

  export async function reconnect() {
    if (!connection) {
      return;
    }

    await connectToSession(connection, true);
  }

  export async function disconnect() {
    const sessionId = currentSessionId;
    if (sessionId) {
      try {
        await disconnectSession(sessionId);
      } catch {
        // The socket close path may already have released the backend session.
      }
    }

    cleanup(false);
    setState('closed');
  }

  function mountTerminal() {
    terminal = new Terminal({
      allowProposedApi: false,
      convertEol: true,
      cursorBlink: true,
      cursorStyle: preferences?.cursorStyle ?? 'block',
      fontFamily: preferences?.fontFamily ?? 'JetBrains Mono, Cascadia Code, monospace',
      fontSize: preferences?.fontSize ?? 14,
      scrollback: 5000,
      theme: terminalTheme(),
    });
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.onData((data) => socket?.sendInput(data));
    terminal.open(host);
    terminal.writeln('SypherTerm terminal ready.');
    terminal.writeln('Select a profile, enter temporary credentials, and connect.');
    scheduleFit();
  }

  async function connectToSession(target: TerminalConnection, force = false) {
    if (!force && target.sessionId === currentSessionId && state === 'connected') {
      return;
    }

    cleanup(false);
    currentSessionId = target.sessionId;
    terminal?.reset();
    setState('connecting');

    socket = new TerminalSocket(target, {
      onData: (data) => terminal?.write(decoder.decode(data, { stream: true })),
      onStatus: (nextState) => {
        setState(nextState === 'closed' ? 'closed' : 'connected', nextState);
      },
      onError: (message) => {
        terminal?.writeln(`\r\n${message}`);
        setState('failed', message);
      },
      onStateChange: (nextState) => setState(nextState),
    });

    try {
      await socket.connect();
      terminal?.focus();
      scheduleFit();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      terminal?.writeln(`\r\n${message}`);
      setState('failed', message);
    }
  }

  function cleanup(resetSession: boolean) {
    if (resizeTimer) {
      clearTimeout(resizeTimer);
      resizeTimer = null;
    }

    socket?.close();
    socket = null;
    lastResize = '';

    if (resetSession) {
      currentSessionId = '';
    }
  }

  function applyPreferences() {
    if (!terminal || !preferences) {
      return;
    }

    terminal.options.fontFamily = preferences.fontFamily;
    terminal.options.fontSize = preferences.fontSize;
    terminal.options.cursorStyle = preferences.cursorStyle;
    terminal.options.theme = terminalTheme();
    scheduleFit();
  }

  function scheduleFit() {
    if (resizeTimer) {
      clearTimeout(resizeTimer);
    }

    resizeTimer = setTimeout(() => {
      void fitAndNotify();
    }, 120);
  }

  async function fitAndNotify() {
    if (!terminal || !fitAddon) {
      return;
    }

    fitAddon.fit();
    const resizeKey = `${terminal.cols}x${terminal.rows}`;
    if (!currentSessionId || resizeKey === lastResize || state !== 'connected') {
      return;
    }

    lastResize = resizeKey;
    try {
      await resizeSession({
        sessionId: currentSessionId,
        cols: terminal.cols,
        rows: terminal.rows,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setState('failed', message);
    }
  }

  function setState(nextState: TerminalSocketState | 'idle', message = '') {
    state = nextState;
    dispatch('status', { state, message });
  }

  function terminalTheme() {
    const light = preferences?.theme === 'light';
    return light
      ? {
          background: '#f8fafc',
          foreground: '#0f172a',
          cursor: '#0f172a',
          selectionBackground: '#cbd5e1',
        }
      : {
          background: '#05070a',
          foreground: '#e6edf3',
          cursor: '#37d0a8',
          selectionBackground: '#263241',
        };
  }
</script>

<div class="terminal-shell min-h-0 flex-1 bg-black">
  <div bind:this={host} class="terminal-host h-full w-full overflow-hidden"></div>
</div>

<style>
  .terminal-shell {
    min-height: 280px;
  }

  .terminal-host :global(.xterm) {
    height: 100%;
    padding: 10px;
  }

  .terminal-host :global(.xterm-viewport) {
    overflow-y: auto;
  }
</style>
