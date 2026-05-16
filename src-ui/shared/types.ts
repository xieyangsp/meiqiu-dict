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
