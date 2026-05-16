<script setup lang="ts">
// Dictionary definition popup. Listens for lookup-request events emitted
// by the backend after the user clicks the floater, queries the bundled
// dictionary, and renders the entry.

import { onMounted, onUnmounted, ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { dictLookup, hidePopup, onLookupRequest } from '../shared/ipc';
import type { DictEntry } from '../shared/types';

const query = ref('');
const entry = ref<DictEntry | null>(null);
const loading = ref(false);
const errorMessage = ref('');
let unlisten: UnlistenFn | null = null;

async function lookup(text: string) {
  query.value = text;
  entry.value = null;
  errorMessage.value = '';
  if (!text) {
    return;
  }
  loading.value = true;
  try {
    entry.value = await dictLookup(text);
  } catch (e) {
    errorMessage.value = String(e);
  } finally {
    loading.value = false;
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    void hidePopup();
  }
}

onMounted(async () => {
  unlisten = await onLookupRequest((payload) => {
    void lookup(payload.text);
  });
  window.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
  }
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <section
    class="flex h-full w-full flex-col overflow-hidden rounded-lg bg-white text-sm text-neutral-900 shadow-xl ring-1 ring-black/10"
  >
    <header class="flex items-center justify-between border-b border-neutral-200 px-3 py-2">
      <span
        data-tauri-drag-region
        class="flex-1 select-none truncate font-medium text-neutral-700"
        >{{ query || '词典' }}</span
      >
      <button
        class="ml-2 rounded px-2 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800"
        title="关闭"
        @click="hidePopup"
      >
        ×
      </button>
    </header>
    <div class="flex-1 overflow-auto px-3 py-2">
      <p v-if="loading" class="text-neutral-500">查询中…</p>
      <p v-else-if="errorMessage" class="text-red-600">{{ errorMessage }}</p>
      <template v-else-if="entry">
        <h2 class="text-base font-semibold text-neutral-900">{{ entry.word }}</h2>
        <p v-if="entry.phonetic" class="mt-0.5 text-xs text-neutral-500">/{{ entry.phonetic }}/</p>
        <pre
          class="mt-2 whitespace-pre-wrap break-words font-sans text-sm text-neutral-800"
        >{{ entry.translation }}</pre>
      </template>
      <p v-else-if="query" class="text-neutral-500">未找到 “{{ query }}” 的释义</p>
      <p v-else class="text-neutral-400">等待查询…</p>
    </div>
  </section>
</template>
