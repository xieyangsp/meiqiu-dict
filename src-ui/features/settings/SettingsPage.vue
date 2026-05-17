<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';

import { getConfig, setAutostart, setConfig } from '../../shared/ipc';
import type { AppConfig, CaptureMethod } from '../../shared/types';
import { SKINS, resolveSkin, type SkinId } from '../../shared/skins';
import { applySkin } from '../../shared/theme';

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

const currentSkin = computed(() => resolveSkin(config.value?.skin));

onMounted(async () => {
  try {
    const cfg = await getConfig();
    config.value = cfg;
    originalAutostart.value = cfg.autostart;
    applySkin(resolveSkin(cfg.skin).id);
  } catch (e) {
    errorMessage.value = String(e);
  }
});

function markDirty() {
  isDirty.value = true;
  successMessage.value = '';
}

function onAutostartToggle() {
  if (!config.value) return;
  config.value.autostart = !config.value.autostart;
  dirtyAutostart.value = config.value.autostart !== originalAutostart.value;
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

function pickSkin(id: SkinId) {
  if (!config.value || config.value.skin === id) return;
  config.value.skin = id;
  applySkin(id);
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
    applySkin(resolveSkin(cfg.skin).id);
  } catch (e) {
    errorMessage.value = String(e);
  }
}
</script>

<template>
  <section
    class="settings-panel mx-auto my-6 max-w-2xl overflow-hidden rounded-xl text-sm"
    style="box-shadow: var(--shadow-popup)"
  >
    <div class="settings-hero flex items-center gap-3 px-6 py-5">
      <div
        class="flex-shrink-0 overflow-hidden rounded-full"
        style="
          width: 56px;
          height: 56px;
          border: 2px solid color-mix(in srgb, var(--settings-hero-fg) 35%, transparent);
          background: color-mix(in srgb, var(--settings-hero-fg) 14%, transparent);
        "
      >
        <img
          :src="currentSkin.teddy"
          :alt="currentSkin.label"
          class="block h-full w-full object-cover"
        />
      </div>
      <div>
        <h1 class="text-lg font-semibold leading-tight">煤球词典</h1>
        <p class="text-xs opacity-75">离线英汉词典 · 划词即查</p>
      </div>
    </div>

    <p v-if="!config" class="px-6 py-8 text-center settings-section-title">加载中…</p>

    <div v-else class="space-y-6 px-6 py-5">
      <div class="space-y-2">
        <label for="hotkey" class="settings-section-title block text-xs font-semibold uppercase tracking-wide">
          全局热键
        </label>
        <input
          id="hotkey"
          v-model="config.hotkey"
          type="text"
          class="settings-input w-full rounded-lg px-3 py-2 font-mono text-sm"
          placeholder="CommandOrControl+Alt+T"
          @input="markDirty"
        />
        <p class="settings-section-title text-xs">
          支持的修饰键：<code>CommandOrControl</code>、<code>Alt</code>、<code>Shift</code>、<code>Super</code>。区分大小写。
        </p>
      </div>

      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm font-medium">开机自启</p>
          <p class="settings-section-title text-xs">Windows 登录后自动启动并常驻托盘</p>
        </div>
        <button
          type="button"
          class="switch"
          :class="{ on: config.autostart }"
          :aria-checked="config.autostart"
          role="switch"
          @click="onAutostartToggle"
        />
      </div>

      <div class="space-y-3">
        <p class="settings-section-title text-xs font-semibold uppercase tracking-wide">皮肤</p>
        <div class="flex flex-wrap items-center gap-2">
          <button
            v-for="s in SKINS"
            :key="s.id"
            type="button"
            class="skin-dot"
            :class="{ selected: config.skin === s.id }"
            :style="{ background: s.swatch }"
            :title="s.label"
            @click="pickSkin(s.id)"
          />
          <span class="settings-section-title ml-2 text-xs">{{ currentSkin.label }}</span>
        </div>
      </div>

      <div class="space-y-3">
        <p class="settings-section-title text-xs font-semibold uppercase tracking-wide">
          划词捕获方式
        </p>
        <label class="flex items-center gap-2 text-sm">
          <input
            v-model="config.uia_enabled"
            type="checkbox"
            class="h-4 w-4"
            @change="markDirty"
          />
          <span>启用 UIA（推荐，无副作用）</span>
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input
            v-model="config.clipboard_enabled"
            type="checkbox"
            class="h-4 w-4"
            @change="markDirty"
          />
          <span>启用剪贴板（模拟 Ctrl+C）</span>
        </label>

        <div>
          <p class="settings-section-title text-xs">优先级顺序（从上到下尝试）：</p>
          <ul class="mt-2 space-y-1">
            <li
              v-for="(method, idx) in sortedCaptureMethods"
              :key="method"
              class="flex items-center justify-between rounded-lg px-3 py-1.5"
              style="border: 1px solid var(--popup-border)"
            >
              <span class="font-mono text-xs">{{ method }}</span>
              <span class="space-x-1">
                <button
                  type="button"
                  class="settings-section-title rounded px-2 text-xs disabled:opacity-30"
                  :disabled="idx === 0"
                  @click="moveMethod(method, -1)"
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="settings-section-title rounded px-2 text-xs disabled:opacity-30"
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

      <div
        class="flex items-center justify-between pt-4"
        style="border-top: 1px solid var(--popup-border)"
      >
        <p v-if="errorMessage" class="text-red-600">{{ errorMessage }}</p>
        <p v-else-if="successMessage" style="color: var(--accent)">{{ successMessage }}</p>
        <p v-else class="settings-section-title">&nbsp;</p>
        <div class="space-x-2">
          <button type="button" class="btn-secondary" :disabled="saving" @click="reload">
            重置
          </button>
          <button type="button" class="btn-primary" :disabled="saving || !isDirty" @click="save">
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
