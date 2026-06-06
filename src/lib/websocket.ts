export type DataPlaneConnectionState = 'stopped' | 'starting' | 'running';

export type TerminalConnection = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};

export type DataPlaneAuthStatus = {
  event: 'status';
  state: 'connected' | string;
  sessionId: string;
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
