<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { dictLookup, hidePopup, onLookupRequest, onSkinChanged, speakText } from '../shared/ipc';
import type { DictEntry } from '../shared/types';
import { resolveSkin } from '../shared/skins';
import { applySkin } from '../shared/theme';
import { getConfig } from '../shared/ipc';

const query = ref('');
const entry = ref<DictEntry | null>(null);
const loading = ref(false);
const speaking = ref<'en_us' | 'en_gb' | null>(null);
const errorMessage = ref('');
const skinTeddy = ref<string>(resolveSkin(null).teddy);
let unlisten: UnlistenFn | null = null;
let unlistenSkin: UnlistenFn | null = null;

const translationLines = computed(() => {
  if (!entry.value) return [] as { pos: string | null; body: string }[];
  return entry.value.translation
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => {
      const match = line.match(/^([a-zA-Z]+\.)\s*(.*)$/);
      if (match) {
        return { pos: match[1], body: match[2] };
      }
      return { pos: null, body: line };
    });
});

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

async function speak(accent: 'en_us' | 'en_gb') {
  const text = entry.value?.word || query.value;
  if (!text) {
    return;
  }
  speaking.value = accent;
  try {
    await speakText(text, accent);
  } catch (e) {
    errorMessage.value = String(e);
  } finally {
    speaking.value = null;
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
  unlistenSkin = await onSkinChanged((skin) => {
    const resolved = resolveSkin(skin);
    skinTeddy.value = resolved.teddy;
    applySkin(resolved.id);
  });
  window.addEventListener('keydown', onKeydown);
  try {
    const cfg = await getConfig();
    skinTeddy.value = resolveSkin(cfg.skin).teddy;
  } catch {
    /* keep default teddy */
  }
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
  }
  if (unlistenSkin) {
    unlistenSkin();
  }
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <section
    class="popup-panel flex h-full w-full flex-col overflow-hidden rounded-xl text-sm"
    style="box-shadow: var(--shadow-popup)"
  >
    <header
      data-tauri-drag-region
      class="popup-header flex select-none items-center justify-between border-b px-3 py-2"
    >
      <div class="flex min-w-0 flex-1 items-center gap-2">
        <div
          class="flex-shrink-0 overflow-hidden rounded-full"
          style="width: 22px; height: 22px; border: 1px solid var(--popup-border)"
        >
          <img :src="skinTeddy" alt="teddy" class="block h-full w-full object-cover" />
        </div>
        <span class="truncate text-sm font-medium">{{ query || '词典' }}</span>
      </div>
      <button
        class="rounded p-1 hover:opacity-70"
        title="关闭"
        @click="hidePopup"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="width: 14px; height: 14px">
          <path d="M6 6 L18 18 M6 18 L18 6" />
        </svg>
      </button>
    </header>
    <div class="flex-1 overflow-auto px-4 py-3">
      <p v-if="loading" class="popup-meta">查询中…</p>
      <p v-else-if="errorMessage" class="text-red-600">{{ errorMessage }}</p>
      <template v-else-if="entry">
        <div class="mb-1 flex items-baseline justify-between gap-3">
          <h2 class="text-xl font-bold tracking-tight" style="color: var(--popup-fg)">
            {{ entry.word }}
          </h2>
          <div class="flex flex-shrink-0 items-center gap-1.5">
            <button
              type="button"
              class="btn-pronounce"
              :disabled="speaking !== null"
              title="英音"
              @click="speak('en_gb')"
            >
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
                style="width: 11px; height: 11px"
              >
                <path d="M3 9v6h4l5 5V4L7 9H3z" />
                <path
                  d="M16.5 8.5 a4 4 0 0 1 0 7"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                />
              </svg>
              {{ speaking === 'en_gb' ? '英…' : '英' }}
            </button>
            <button
              type="button"
              class="btn-pronounce"
              :disabled="speaking !== null"
              title="美音"
              @click="speak('en_us')"
            >
              <svg
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
                style="width: 11px; height: 11px"
              >
                <path d="M3 9v6h4l5 5V4L7 9H3z" />
                <path
                  d="M16.5 8.5 a4 4 0 0 1 0 7"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                />
                <path
                  d="M19 6.5 a7 7 0 0 1 0 11"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                />
              </svg>
              {{ speaking === 'en_us' ? '美…' : '美' }}
            </button>
          </div>
        </div>
        <p v-if="entry.phonetic" class="popup-meta mb-3 font-mono text-xs">
          /{{ entry.phonetic }}/
        </p>
        <div class="space-y-1.5 text-[13px] leading-relaxed" style="color: var(--popup-fg)">
          <p v-for="(line, idx) in translationLines" :key="idx">
            <span
              v-if="line.pos"
              class="mr-1 font-semibold"
              style="color: var(--accent)"
              >{{ line.pos }}</span
            >
            <span>{{ line.body }}</span>
          </p>
        </div>
      </template>
      <p v-else-if="query" class="popup-meta">未找到 “{{ query }}” 的释义</p>
      <p v-else class="popup-meta">等待查询…</p>
    </div>
  </section>
</template>
