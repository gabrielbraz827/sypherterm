export type DataPlaneConnectionState = 'stopped' | 'starting' | 'running';

export type TerminalConnection = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};

export type DataPlaneAuthStatus = {
  event: 'status';
  state: 'connected' | string;
  sessionId?: string;
};

export type TerminalSocketState = 'connecting' | 'connected' | 'closed' | 'failed';

export type TerminalSocketHandlers = {
  onData?: (data: Uint8Array) => void;
  onStatus?: (state: string) => void;
  onError?: (message: string) => void;
  onStateChange?: (state: TerminalSocketState) => void;
};

export function authenticateDataPlaneSession(connection: TerminalConnection): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(connection.wsUrl);

    socket.addEventListener(
      'open',
      () => {
        socket.send(
          JSON.stringify({
            event: 'auth',
            token: connection.authToken,
          }),
        );
      },
      { once: true },
    );

    socket.addEventListener(
      'message',
      (event) => {
        if (typeof event.data !== 'string') {
          return;
        }

        const payload = JSON.parse(event.data) as DataPlaneAuthStatus | { event: 'error'; message: string };
        if (payload.event === 'status' && payload.state === 'connected') {
          resolve(socket);
          return;
        }

        if (payload.event === 'error') {
          socket.close();
          reject(new Error(payload.message));
        }
      },
      { once: true },
    );

    socket.addEventListener(
      'error',
      () => {
        reject(new Error('failed to connect to local Data Plane'));
      },
      { once: true },
    );
  });
}

export class TerminalSocket {
  private socket: WebSocket | null = null;

  constructor(
    private readonly connection: TerminalConnection,
    private readonly handlers: TerminalSocketHandlers = {},
  ) {}

  async connect() {
    this.setState('connecting');
    const socket = await authenticateDataPlaneSession(this.connection);
    this.socket = socket;
    socket.binaryType = 'arraybuffer';

    socket.addEventListener('message', (event) => this.handleMessage(event));
    socket.addEventListener('close', () => this.setState('closed'));
    socket.addEventListener('error', () => {
      this.handlers.onError?.('local Data Plane socket failed');
      this.setState('failed');
    });

    this.setState('connected');
  }

  sendInput(input: string) {
    if (this.socket?.readyState !== WebSocket.OPEN) {
      return;
    }

    this.socket.send(new TextEncoder().encode(input));
  }

  close() {
    this.socket?.close();
    this.socket = null;
  }

  private handleMessage(event: MessageEvent) {
    if (typeof event.data === 'string') {
      this.handleTextMessage(event.data);
      return;
    }

    if (event.data instanceof ArrayBuffer) {
      this.handlers.onData?.(new Uint8Array(event.data));
      return;
    }

    if (event.data instanceof Blob) {
      void event.data.arrayBuffer().then((buffer) => {
        this.handlers.onData?.(new Uint8Array(buffer));
      });
    }
  }

  private handleTextMessage(data: string) {
    let payload: DataPlaneAuthStatus | { event?: string; message?: string } | null = null;
    try {
      payload = JSON.parse(data) as DataPlaneAuthStatus | { event?: string; message?: string };
    } catch {
      return;
    }

    if (payload.event === 'status' && 'state' in payload) {
      this.handlers.onStatus?.(payload.state);
      return;
    }

    if (payload.event === 'error') {
      this.handlers.onError?.(payload.message ?? 'terminal socket error');
    }
  }

  private setState(state: TerminalSocketState) {
    this.handlers.onStateChange?.(state);
  }
}
