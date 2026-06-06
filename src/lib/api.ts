import { invoke } from '@tauri-apps/api/core';

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

export function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>('get_app_status');
}

export function createVault(request: CreateVaultRequest): Promise<VaultStatus> {
  return invoke<VaultStatus>('create_vault', { request });
}

export function unlockVault(request: UnlockVaultRequest): Promise<VaultStatus> {
  return invoke<VaultStatus>('unlock_vault', { request });
}

export function lockVault(): Promise<VaultStatus> {
  return invoke<VaultStatus>('lock_vault');
}

export function changeMasterPassword(
  request: ChangeMasterPasswordRequest,
): Promise<VaultStatus> {
  return invoke<VaultStatus>('change_master_password', { request });
}

export function listProfiles(): Promise<ConnectionProfileSummary[]> {
  return invoke<ConnectionProfileSummary[]>('list_profiles');
}

export function saveProfile(profile: ConnectionProfileDraft): Promise<ConnectionProfile> {
  return invoke<ConnectionProfile>('save_profile', { profile });
}

export function deleteProfile(id: string): Promise<DeleteResult> {
  return invoke<DeleteResult>('delete_profile', { id });
}

export function getPreferences(): Promise<UserPreferences> {
  return invoke<UserPreferences>('get_preferences');
}

export function savePreferences(preferences: UserPreferences): Promise<UserPreferences> {
  return invoke<UserPreferences>('save_preferences', { preferences });
}
