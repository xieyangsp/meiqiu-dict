<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import SettingsPage from '../features/settings/SettingsPage.vue';

let unlistenClose: UnlistenFn | null = null;

onMounted(async () => {
  const win = getCurrentWindow();
  unlistenClose = await win.onCloseRequested(async (event) => {
    event.preventDefault();
    await win.hide();
  });
});

onUnmounted(() => {
  if (unlistenClose) {
    unlistenClose();
  }
});
</script>

<template>
  <main class="h-full overflow-auto" style="background: var(--settings-bg)">
    <SettingsPage />
  </main>
</template>
