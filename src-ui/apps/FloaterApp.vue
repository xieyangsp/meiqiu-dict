<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';

import { hideFloater, onSelectionAcquired, requestLookup } from '../shared/ipc';

const AUTO_HIDE_MS = 3000;

const text = ref('');
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
      class="rounded-full bg-neutral-900/85 px-3 py-1 text-xs text-white shadow hover:bg-neutral-700"
      @click="onClick"
    >
      查
    </button>
  </div>
</template>
