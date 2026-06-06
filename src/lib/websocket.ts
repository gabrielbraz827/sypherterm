export type DataPlaneConnectionState = 'stopped' | 'starting' | 'running';

export type TerminalConnection = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};
