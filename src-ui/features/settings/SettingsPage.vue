<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import { getConfig, setAutostart, setConfig } from '../../shared/ipc';
import type { AppConfig, CaptureMethod } from '../../shared/types';

const config = ref<AppConfig | null>(null);
const saving = ref(false);
const errorMessage = ref('');
const successMessage = ref('');

const isDirty = ref(false);
const dirtyAutostart = ref(false);
const originalAutostart = ref(false);

const sortedCaptureMethods = computed<CaptureMethod[]>(() => {
  if (!config.value) return [];
  return config.value.capture_methods;
});

onMounted(async () => {
  try {
    const cfg = await getConfig();
    config.value = cfg;
    originalAutostart.value = cfg.autostart;
  } catch (e) {
    errorMessage.value = String(e);
  }
});

function markDirty() {
  isDirty.value = true;
  successMessage.value = '';
}

function onAutostartToggle(event: Event) {
  if (!config.value) return;
  const next = (event.target as HTMLInputElement).checked;
  config.value.autostart = next;
  dirtyAutostart.value = next !== originalAutostart.value;
  markDirty();
}

function moveMethod(method: CaptureMethod, direction: -1 | 1) {
  if (!config.value) return;
  const list = config.value.capture_methods.slice();
  const idx = list.indexOf(method);
  const target = idx + direction;
  if (idx < 0 || target < 0 || target >= list.length) return;
  [list[idx], list[target]] = [list[target], list[idx]];
  config.value.capture_methods = list;
  markDirty();
}

async function save() {
  if (!config.value) return;
  saving.value = true;
  errorMessage.value = '';
  successMessage.value = '';
  try {
    if (dirtyAutostart.value) {
      await setAutostart(config.value.autostart);
      originalAutostart.value = config.value.autostart;
      dirtyAutostart.value = false;
    }
    await setConfig(config.value);
    successMessage.value = '已保存';
    isDirty.value = false;
  } catch (e) {
    errorMessage.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function reload() {
  errorMessage.value = '';
  successMessage.value = '';
  try {
    const cfg = await getConfig();
    config.value = cfg;
    originalAutostart.value = cfg.autostart;
    dirtyAutostart.value = false;
    isDirty.value = false;
  } catch (e) {
    errorMessage.value = String(e);
  }
}
</script>

<template>
  <section class="mx-auto max-w-2xl space-y-6 p-6 text-sm text-neutral-900">
    <header>
      <h1 class="text-xl font-semibold">煤球词典 · 设置</h1>
      <p class="mt-1 text-xs text-neutral-500">修改后点击保存。热键变更会立即生效。</p>
    </header>

    <p v-if="!config" class="text-neutral-500">加载中…</p>

    <div v-else class="space-y-6">
      <div class="space-y-2">
        <label for="hotkey" class="block font-medium">全局热键</label>
        <input
          id="hotkey"
          v-model="config.hotkey"
          type="text"
          class="w-full rounded border border-neutral-300 px-3 py-1.5 font-mono text-sm focus:border-neutral-500 focus:outline-none"
          placeholder="CommandOrControl+Alt+T"
          @input="markDirty"
        />
        <p class="text-xs text-neutral-500">
          支持的修饰键：<code>CommandOrControl</code>、<code>Alt</code>、<code>Shift</code>、<code>Super</code>。区分大小写。
        </p>
      </div>

      <div class="flex items-center justify-between">
        <div>
          <p class="font-medium">开机自启</p>
          <p class="text-xs text-neutral-500">Windows 登录后自动启动并常驻托盘。</p>
        </div>
        <label class="inline-flex cursor-pointer items-center">
          <input
            type="checkbox"
            class="h-4 w-4"
            :checked="config.autostart"
            @change="onAutostartToggle"
          />
        </label>
      </div>

      <div class="space-y-3">
        <p class="font-medium">划词捕获方式</p>
        <div class="flex items-center justify-between">
          <label class="inline-flex items-center gap-2">
            <input
              v-model="config.uia_enabled"
              type="checkbox"
              class="h-4 w-4"
              @change="markDirty"
            />
            <span>启用 UIA（推荐，无副作用）</span>
          </label>
        </div>
        <div class="flex items-center justify-between">
          <label class="inline-flex items-center gap-2">
            <input
              v-model="config.clipboard_enabled"
              type="checkbox"
              class="h-4 w-4"
              @change="markDirty"
            />
            <span>启用剪贴板（模拟 Ctrl+C）</span>
          </label>
        </div>

        <div>
          <p class="text-xs text-neutral-500">优先级顺序（从上到下尝试）：</p>
          <ul class="mt-2 space-y-1">
            <li
              v-for="(method, idx) in sortedCaptureMethods"
              :key="method"
              class="flex items-center justify-between rounded border border-neutral-200 px-3 py-1.5"
            >
              <span class="font-mono text-xs">{{ method }}</span>
              <span class="space-x-1">
                <button
                  type="button"
                  class="rounded px-2 text-xs text-neutral-500 hover:bg-neutral-100 disabled:opacity-30"
                  :disabled="idx === 0"
                  @click="moveMethod(method, -1)"
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="rounded px-2 text-xs text-neutral-500 hover:bg-neutral-100 disabled:opacity-30"
                  :disabled="idx === sortedCaptureMethods.length - 1"
                  @click="moveMethod(method, 1)"
                >
                  ↓
                </button>
              </span>
            </li>
          </ul>
        </div>
      </div>

      <div class="flex items-center justify-between border-t border-neutral-200 pt-4">
        <p v-if="errorMessage" class="text-red-600">{{ errorMessage }}</p>
        <p v-else-if="successMessage" class="text-green-600">{{ successMessage }}</p>
        <p v-else class="text-neutral-400">&nbsp;</p>
        <div class="space-x-2">
          <button
            type="button"
            class="rounded border border-neutral-300 px-3 py-1.5 text-neutral-700 hover:bg-neutral-100"
            :disabled="saving"
            @click="reload"
          >
            重置
          </button>
          <button
            type="button"
            class="rounded bg-neutral-900 px-3 py-1.5 text-white hover:bg-neutral-700 disabled:opacity-50"
            :disabled="saving || !isDirty"
            @click="save"
          >
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
