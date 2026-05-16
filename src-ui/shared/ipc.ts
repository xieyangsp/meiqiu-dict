import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

import type { DictEntry, LookupPayload, SelectionPayload } from './types';

export function onSelectionAcquired(
  cb: (payload: SelectionPayload) => void,
): Promise<UnlistenFn> {
  return listen<SelectionPayload>('selection-acquired', (event) => cb(event.payload));
}

export function onLookupRequest(
  cb: (payload: LookupPayload) => void,
): Promise<UnlistenFn> {
  return listen<LookupPayload>('lookup-request', (event) => cb(event.payload));
}

export function requestLookup(text: string): Promise<void> {
  return invoke('request_lookup', { text });
}

export function dictLookup(word: string): Promise<DictEntry | null> {
  return invoke<DictEntry | null>('dict_lookup', { word });
}

export function hideCurrentWindow(): Promise<void> {
  return getCurrentWebviewWindow().hide();
}

export async function hideFloater(): Promise<void> {
  await getCurrentWebviewWindow().hide();
  await invoke('notify_floater_hidden');
}

export async function hidePopup(): Promise<void> {
  await getCurrentWebviewWindow().hide();
  await invoke('notify_popup_hidden');
}
