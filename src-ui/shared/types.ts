// Mirror Rust serde structs one-to-one.

export interface SelectionPayload {
  text: string;
}

export interface LookupPayload {
  text: string;
}

export interface DictEntry {
  word: string;
  phonetic: string;
  translation: string;
  lang_pair: string;
}

export type CaptureMethod = 'uia' | 'clipboard';

export interface AppConfig {
  hotkey: string;
  autostart: boolean;
  tts_voice: string | null;
  uia_enabled: boolean;
  clipboard_enabled: boolean;
  capture_methods: CaptureMethod[];
  skin: string;
}
