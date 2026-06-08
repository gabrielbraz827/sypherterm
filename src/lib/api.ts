import { invoke } from '@tauri-apps/api/core';

export type CommandError = {
  code: string;
  message: string;
  recoverable: boolean;
};

export type VaultState = 'missing' | 'locked' | 'unlocked';
export type DataPlaneState = 'stopped' | 'starting' | 'running';

export type AppStatus = {
  appVersion: string;
  vault: VaultState;
  activeSessions: number;
  dataPlane: DataPlaneState;
};

export type VaultStatus = {
  state: VaultState;
  version?: number;
};

export type CreateVaultRequest = {
  masterPassword: string;
};

export type UnlockVaultRequest = {
  masterPassword: string;
};

export type ChangeMasterPasswordRequest = {
  currentPassword: string;
  newPassword: string;
};

export type ConnectionProfile = {
  id: string;
  version: 1;
  name: string;
  host: string;
  port: number;
  username?: string;
  groupId?: string;
  tags: string[];
  credentialRef?: string;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
};

export type ConnectionProfileDraft = {
  id?: string;
  name: string;
  host: string;
  port: number;
  username?: string;
  groupId?: string;
  tags?: string[];
  credentialRef?: string;
};

export type ConnectionProfileSummary = Omit<ConnectionProfile, 'credentialRef'> & {
  hasCredential: boolean;
};

export type ProfileListFilters = {
  query?: string;
  groupId?: string;
  tag?: string;
  recentFirst?: boolean;
};

export type DeleteResult = {
  deleted: boolean;
};

export type UserPreferences = {
  version: 1;
  theme: 'system' | 'dark' | 'light';
  fontFamily: string;
  fontSize: number;
  cursorStyle: 'block' | 'bar' | 'underline';
};

export type ConnectSshRequest = {
  profileId?: string;
  host?: string;
  port?: number;
  username?: string;
  credentialRef?: string;
  password?: string;
  privateKeyPath?: string;
  passphrase?: string;
  cols: number;
  rows: number;
};

export type ConnectSshResponse = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};

export type DataPlaneSession = ConnectSshResponse & {
  expiresAt: string;
};

export type SessionResizeRequest = {
  sessionId: string;
  cols: number;
  rows: number;
};

export type SessionStatus = {
  sessionId: string;
  state: 'connecting' | 'connected' | 'closing' | 'closed' | 'failed' | string;
};

export type RemoteEntryKind = 'directory' | 'file' | 'symlink' | 'other';

export type RemoteDirEntry = {
  name: string;
  path: string;
  kind: RemoteEntryKind;
  size: number;
  permissions?: string;
  modifiedAt?: string;
};

export type SftpPathRequest = {
  sessionId: string;
  path: string;
};

export type SftpTransferRequest = {
  sessionId: string;
  remotePath: string;
  localPath: string;
};

export type SftpRenameRequest = {
  sessionId: string;
  oldPath: string;
  newPath: string;
};

export type SftpDeleteRequest = {
  sessionId: string;
  path: string;
  kind?: RemoteEntryKind;
};

export type SftpCancelRequest = {
  jobId: string;
};

export type TransferJob = {
  jobId: string;
  sessionId: string;
  direction: 'download' | 'upload';
  state: 'running' | 'completed' | 'cancelled' | 'failed';
  remotePath: string;
  localPath: string;
  bytesTransferred: number;
  totalBytes?: number;
  message?: string;
};

export type SyncRequest = {
  providerId: string;
  direction: 'push' | 'pull' | 'bidirectional' | string;
  deviceId?: string;
};

export type SyncProviderKind = 'local';

export type SyncProviderConfig = {
  providerId: string;
  kind: SyncProviderKind;
  localPath: string;
  deviceId?: string;
};

export type SyncProviderStatus = {
  providerId: string;
  kind: string;
  state: string;
  rootPath: string;
};

export type SyncJobStatus = {
  jobId: string;
  state: string;
  versionId?: string;
  payloadHash?: string;
  message?: string;
};

export type SyncVersion = {
  versionId: string;
  deviceId: string;
  payloadHash: string;
  createdAt: string;
};

export type TunnelRequest = {
  sessionId?: string;
  profileId?: string;
  mode: 'local' | 'remote' | 'dynamic' | string;
  bindHost: string;
  bindPort: number;
  targetHost?: string;
  targetPort?: number;
  label?: string;
  allowExternalBind?: boolean;
};

export type TunnelStatus = {
  tunnelId: string;
  sessionId: string;
  mode: 'local' | 'remote' | 'dynamic' | string;
  state: 'starting' | 'running' | 'stopping' | 'stopped' | 'failed' | string;
  bindHost: string;
  bindPort: number;
  targetHost?: string;
  targetPort?: number;
  label?: string;
  startedAt?: string;
  lastError?: string;
};

export type Snippet = {
  id: string;
  version: 1;
  name: string;
  body: string;
  tags: string[];
  variables: string[];
  createdAt: string;
  updatedAt: string;
};

export type SnippetSummary = Omit<Snippet, 'body'>;

export type SnippetDraft = {
  id?: string;
  name: string;
  body: string;
  tags?: string[];
  variables?: string[];
};

export type SnippetFilters = {
  query?: string;
  tag?: string;
};

function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error &&
    'recoverable' in error
  );
}

function normalizeCommandError(error: unknown): CommandError {
  if (isCommandError(error)) {
    return {
      code: String(error.code),
      message: String(error.message),
      recoverable: Boolean(error.recoverable),
    };
  }

  if (error instanceof Error) {
    return {
      code: 'unknown_error',
      message: error.message,
      recoverable: false,
    };
  }

  return {
    code: 'unknown_error',
    message: String(error),
    recoverable: false,
  };
}

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeCommandError(error);
  }
}

export function getAppStatus(): Promise<AppStatus> {
  return invokeCommand<AppStatus>('get_app_status');
}

export function createVault(request: CreateVaultRequest): Promise<VaultStatus> {
  return invokeCommand<VaultStatus>('create_vault', { request });
}

export function unlockVault(request: UnlockVaultRequest): Promise<VaultStatus> {
  return invokeCommand<VaultStatus>('unlock_vault', { request });
}

export function lockVault(): Promise<VaultStatus> {
  return invokeCommand<VaultStatus>('lock_vault');
}

export function changeMasterPassword(
  request: ChangeMasterPasswordRequest,
): Promise<VaultStatus> {
  return invokeCommand<VaultStatus>('change_master_password', { request });
}

export function listProfiles(filters?: ProfileListFilters): Promise<ConnectionProfileSummary[]> {
  return invokeCommand<ConnectionProfileSummary[]>(
    'list_profiles',
    filters ? { filters } : undefined,
  );
}

export function saveProfile(profile: ConnectionProfileDraft): Promise<ConnectionProfile> {
  return invokeCommand<ConnectionProfile>('save_profile', { profile });
}

export function deleteProfile(id: string): Promise<DeleteResult> {
  return invokeCommand<DeleteResult>('delete_profile', { id });
}

export function duplicateProfile(id: string): Promise<ConnectionProfile> {
  return invokeCommand<ConnectionProfile>('duplicate_profile', { id });
}

export function getPreferences(): Promise<UserPreferences> {
  return invokeCommand<UserPreferences>('get_preferences');
}

export function savePreferences(preferences: UserPreferences): Promise<UserPreferences> {
  return invokeCommand<UserPreferences>('save_preferences', { preferences });
}

export function openDataPlaneSession(): Promise<DataPlaneSession> {
  return invokeCommand<DataPlaneSession>('open_data_plane_session');
}

export function connectSsh(request: ConnectSshRequest): Promise<ConnectSshResponse> {
  return invokeCommand<ConnectSshResponse>('connect_ssh', { request });
}

export function disconnectSession(sessionId: string): Promise<SessionStatus> {
  return invokeCommand<SessionStatus>('disconnect_session', { sessionId });
}

export function resizeSession(request: SessionResizeRequest): Promise<SessionStatus> {
  return invokeCommand<SessionStatus>('resize_session', { request });
}

export function sftpListDir(request: SftpPathRequest): Promise<RemoteDirEntry[]> {
  return invokeCommand<RemoteDirEntry[]>('sftp_list_dir', { request });
}

export function sftpDownload(request: SftpTransferRequest): Promise<TransferJob> {
  return invokeCommand<TransferJob>('sftp_download', { request });
}

export function sftpUpload(request: SftpTransferRequest): Promise<TransferJob> {
  return invokeCommand<TransferJob>('sftp_upload', { request });
}

export function sftpCancelTransfer(request: SftpCancelRequest): Promise<TransferJob> {
  return invokeCommand<TransferJob>('sftp_cancel_transfer', { request });
}

export function sftpMkdir(request: SftpPathRequest): Promise<RemoteDirEntry> {
  return invokeCommand<RemoteDirEntry>('sftp_mkdir', { request });
}

export function sftpRename(request: SftpRenameRequest): Promise<RemoteDirEntry> {
  return invokeCommand<RemoteDirEntry>('sftp_rename', { request });
}

export function sftpDelete(request: SftpDeleteRequest): Promise<RemoteDirEntry> {
  return invokeCommand<RemoteDirEntry>('sftp_delete', { request });
}

export function triggerCloudSync(request: SyncRequest): Promise<SyncJobStatus> {
  return invokeCommand<SyncJobStatus>('trigger_cloud_sync', { request });
}

export function testSyncProvider(config: SyncProviderConfig): Promise<SyncProviderStatus> {
  return invokeCommand<SyncProviderStatus>('test_sync_provider', { config });
}

export function listSyncVersions(config: SyncProviderConfig): Promise<SyncVersion[]> {
  return invokeCommand<SyncVersion[]>('list_sync_versions', { config });
}

export function startTunnel(request: TunnelRequest): Promise<TunnelStatus> {
  return invokeCommand<TunnelStatus>('start_tunnel', { request });
}

export function stopTunnel(tunnelId: string): Promise<TunnelStatus> {
  return invokeCommand<TunnelStatus>('stop_tunnel', { tunnelId });
}

export function listTunnels(): Promise<TunnelStatus[]> {
  return invokeCommand<TunnelStatus[]>('list_tunnels');
}

export function listSessionTunnels(sessionId: string): Promise<TunnelStatus[]> {
  return invokeCommand<TunnelStatus[]>('list_session_tunnels', { sessionId });
}

export function listSnippets(filters?: SnippetFilters): Promise<SnippetSummary[]> {
  return invokeCommand<SnippetSummary[]>(
    'list_snippets',
    filters ? { filters } : undefined,
  );
}

export function getSnippet(id: string): Promise<Snippet> {
  return invokeCommand<Snippet>('get_snippet', { id });
}

export function saveSnippet(draft: SnippetDraft): Promise<Snippet> {
  return invokeCommand<Snippet>('save_snippet', { draft });
}

export function deleteSnippet(id: string): Promise<DeleteResult> {
  return invokeCommand<DeleteResult>('delete_snippet', { id });
}
