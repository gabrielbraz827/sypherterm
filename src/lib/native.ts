import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { arch, eol, family, platform, type as osType } from '@tauri-apps/plugin-os';
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

export type NativeEvent =
  | 'connection:connected'
  | 'connection:failed'
  | 'sync:completed'
  | 'sync:failed'
  | 'sync:conflict';

export type NativeOsInfo = {
  platform: string;
  family: string;
  type: string;
  arch: string;
  eol: string;
};

export async function copyTerminalSelection(selection: string) {
  if (!selection) {
    return false;
  }

  await writeText(selection, { label: 'SypherTerm terminal selection' });
  return true;
}

export async function readTerminalClipboard() {
  return readText();
}

export function getNativeOsInfo(): NativeOsInfo {
  return {
    platform: platform(),
    family: family(),
    type: osType(),
    arch: arch(),
    eol: eol(),
  };
}

export async function openExternalUrl(url: string) {
  const parsed = new URL(url);
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    throw new Error('Only http and https URLs can be opened.');
  }

  await openUrl(parsed);
}

export async function revealAuthorizedPath(path: string) {
  const normalized = path.trim();
  if (!normalized) {
    throw new Error('A path is required.');
  }

  await revealItemInDir(normalized);
}

export async function notifyNativeEvent(event: NativeEvent, detail = '') {
  const options = nativeNotificationOptions(event, detail);
  if (!options) {
    return false;
  }

  const granted = await ensureNotificationPermission();
  if (!granted) {
    return false;
  }

  sendNotification(options);
  return true;
}

export function sanitizeNotificationBody(value: string) {
  return value
    .replace(/\b(password|passphrase|privateKey|authToken|token|secret)\s*[:=]\s*\S+/gi, '$1=[redacted]')
    .replace(/\b[a-f0-9]{32,}\b/gi, '[hash]')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 120);
}

function nativeNotificationOptions(event: NativeEvent, detail: string) {
  const body = sanitizeNotificationBody(detail);

  switch (event) {
    case 'connection:connected':
      return {
        title: 'SypherTerm',
        body: body || 'SSH session connected.',
      };
    case 'connection:failed':
      return {
        title: 'SypherTerm',
        body: body || 'SSH session failed.',
      };
    case 'sync:completed':
      return {
        title: 'SypherTerm',
        body: body || 'Sync completed.',
      };
    case 'sync:failed':
      return {
        title: 'SypherTerm',
        body: body || 'Sync failed.',
      };
    case 'sync:conflict':
      return {
        title: 'SypherTerm',
        body: body || 'Sync conflict detected.',
      };
  }
}

async function ensureNotificationPermission() {
  if (await isPermissionGranted()) {
    return true;
  }

  return (await requestPermission()) === 'granted';
}
