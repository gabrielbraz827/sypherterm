import { writable } from 'svelte/store';

import type { AppStatus, ConnectionProfileSummary, UserPreferences, VaultStatus } from './api';

export const appStatus = writable<AppStatus | null>(null);
export const vaultStatus = writable<VaultStatus | null>(null);
export const profiles = writable<ConnectionProfileSummary[]>([]);
export const preferences = writable<UserPreferences | null>(null);
