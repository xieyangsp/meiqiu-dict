// Centralized IPC layer. Components must not call @tauri-apps/api directly.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

import type { SelectionPayload } from './types';

export function onSelectionAcquired(
  cb: (payload: SelectionPayload) => void,
): Promise<UnlistenFn> {
  return listen<SelectionPayload>('selection-acquired', (event) => cb(event.payload));
}

export function hideCurrentWindow(): Promise<void> {
  return getCurrentWebviewWindow().hide();
}
