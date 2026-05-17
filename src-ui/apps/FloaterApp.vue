<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { getConfig, hideFloater, onSelectionAcquired, requestLookup } from '../shared/ipc';
import { resolveSkin } from '../shared/skins';

const AUTO_HIDE_MS = 3000;

const text = ref('');
const skinTeddy = ref<string>(resolveSkin(null).teddy);
let unlisten: UnlistenFn | null = null;
let hideTimer: number | null = null;

function scheduleHide() {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
  }
  hideTimer = window.setTimeout(() => {
    void hideFloater();
  }, AUTO_HIDE_MS);
}

function onClick() {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }
  if (text.value) {
    void requestLookup(text.value);
  } else {
    void hideFloater();
  }
}

onMounted(async () => {
  unlisten = await onSelectionAcquired((payload) => {
    text.value = payload.text;
    scheduleHide();
  });
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
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
  }
});
</script>

<template>
  <div class="flex h-full w-full items-center justify-center">
    <button
      :title="text"
      class="overflow-hidden rounded-full"
      style="
        width: 36px;
        height: 36px;
        padding: 0;
        border: 2px solid var(--brand);
        background: var(--brand);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
      "
      @click="onClick"
    >
      <img :src="skinTeddy" alt="teddy" class="block h-full w-full object-cover" />
    </button>
  </div>
</template>
