<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { theme, toggleTheme } from '../theme'
import {
  detectDevice,
  getAlertTypes,
  logout as apiLogout,
  previewCount,
  radarTypesString,
  toAppError,
  updateDevice,
} from '../api'
import type { AlertType, DeviceInfo, UpdateSummary } from '../types'

const emit = defineEmits<{ (e: 'logout'): void }>()

const alertTypes = ref<AlertType[]>([])
const selected = ref<Set<number>>(new Set())
const devices = ref<DeviceInfo[]>([])
const count = ref<number | null>(null)
const busy = ref(false)
const toast = ref<{ message: string; ok: boolean; visible: boolean }>({ message: '', ok: true, visible: false })
let toastTimer: number | undefined
let timer: number | undefined
let countRequest = 0
let debounceTimer: number | undefined
let scanInFlight = false
const updateAvailable = ref(false)
const updateBusy = ref(false)
const updateHandle = shallowRef<Update | null>(null)

const currentDevice = computed(() => devices.value[0])
const deviceLabel = computed(() => {
  const d = currentDevice.value
  if (!d) return ''
  const parts = d.drive.split(/[/\\]+/).filter(Boolean)
  return parts[parts.length - 1] || d.drive
})
const radarTypes = computed(() => radarTypesString([...selected.value]))
const hasSelection = computed(() => radarTypes.value !== null)

function showToast(msg: string, ok: boolean) {
  window.clearTimeout(toastTimer)
  toast.value = { message: msg, ok, visible: true }
  toastTimer = window.setTimeout(() => {
    toast.value.visible = false
  }, 5000)
}

function dismissToast() {
  window.clearTimeout(toastTimer)
  toast.value.visible = false
}

function toggle(code: number) {
  const next = new Set(selected.value)
  if (next.has(code)) next.delete(code)
  else next.add(code)
  selected.value = next
}

async function refreshCount() {
  const req = ++countRequest
  if (radarTypes.value === null) {
    count.value = null
    return
  }
  try {
    const value = await previewCount(radarTypes.value)
    if (req === countRequest) count.value = value
  } catch {
    if (req === countRequest) count.value = null
  }
}

function debouncedRefreshCount() {
  window.clearTimeout(debounceTimer)
  debounceTimer = window.setTimeout(refreshCount, 250)
}

async function refreshDevices() {
  if (scanInFlight) return
  scanInFlight = true
  try {
    devices.value = await detectDevice()
  } catch {
    // transient detect failure: keep previous device state, don't flip UI
  } finally {
    scanInFlight = false
  }
}

async function doUpdate() {
  if (!currentDevice.value || radarTypes.value === null || busy.value) return
  busy.value = true
  dismissToast()
  try {
    const s: UpdateSummary = await updateDevice(currentDevice.value.kind, radarTypes.value)
    showToast(`Arquivo atualizado: ${s.filesWritten.join(', ')}`, true)
  } catch (e) {
    const err = toAppError(e)
    if (err.kind === 'unauthorized') {
      apiLogout()
      emit('logout')
      return
    }
    showToast(err.message, false)
  } finally {
    busy.value = false
  }
}

async function checkForUpdate() {
  try {
    const update = await check()
    updateHandle.value = update
    updateAvailable.value = !!update
  } catch {
    updateHandle.value = null
    updateAvailable.value = false
  }
}

async function installUpdate() {
  updateBusy.value = true
  try {
    if (updateHandle.value) {
      await updateHandle.value.downloadAndInstall()
      await relaunch()
    }
  } catch (e) {
    updateHandle.value = null
    showToast(toAppError(e).message, false)
  } finally {
    updateBusy.value = false
  }
}

function doLogout() {
  apiLogout()
  emit('logout')
}

onMounted(async () => {
  try {
    alertTypes.value = await getAlertTypes()
  } catch {
    showToast('Não foi possível carregar os tipos de alerta.', false)
  }
  selected.value = new Set(alertTypes.value.filter((t) => t.default).map((t) => t.code))
  refreshDevices()
  timer = window.setInterval(refreshDevices, 2000)
  checkForUpdate()
})

onUnmounted(() => {
  window.clearInterval(timer)
  window.clearTimeout(debounceTimer)
  window.clearTimeout(toastTimer)
})

watch(radarTypes, debouncedRefreshCount)
</script>

<template>
  <div class="main">
    <header>
      <img src="/logo.svg" alt="MapaRadar" class="logo-sm" />
      <nav>
        <button class="link theme-toggle" :aria-label="theme === 'dark' ? 'Alternar para tema claro' : 'Alternar para tema escuro'" @click="toggleTheme">
          <svg v-if="theme === 'dark'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
        </button>
        <button class="link" @click="openUrl('https://maparadar.com/atualizador/')">Sobre</button>
        <button class="link" @click="doLogout">Sair</button>
      </nav>
    </header>

    <section v-if="updateAvailable" class="card update-banner">
      <span>Nova versão disponível</span>
      <button class="primary small" :disabled="updateBusy || busy" @click="installUpdate">
        {{ updateBusy ? 'Atualizando…' : 'Atualizar' }}
      </button>
    </section>

    <section class="card device">
      <template v-if="currentDevice">
        <img :src="`/${currentDevice.kind}.svg`" :alt="currentDevice.display" class="device-logo" />
        <div class="device-drive">{{ deviceLabel }}</div>
      </template>
      <div v-else class="device-waiting">
        <h2 class="device-waiting-title">Aguardando GPS...</h2>
        <span class="spinner spinner-lg"></span>
      </div>
    </section>

    <section class="card">
      <h2>Tipos de alerta</h2>
      <div class="pills">
        <label
          v-for="t in alertTypes"
          :key="t.code"
          class="pill"
          :class="{ active: selected.has(t.code) }"
        >
          <input type="checkbox" :checked="selected.has(t.code)" @change="toggle(t.code)" />
          <img :src="`/icons/${t.icon}.svg`" :alt="t.label" />
          <span>{{ t.label }}</span>
        </label>
      </div>
      <p v-if="alertTypes.length === 0" class="hint">Não foi possível carregar os tipos de alerta.</p>
      <p class="count">Pontos a exportar: <strong>{{ count ?? '—' }}</strong></p>
    </section>

    <section class="actions">
      <button
        class="primary"
        :disabled="!currentDevice || !hasSelection || busy || updateBusy"
        @click="doUpdate"
      >
        {{ busy ? 'Atualizando…' : 'Atualizar dispositivo' }}
      </button>
    </section>

    <Transition name="toast">
      <div v-if="toast.visible" class="toast" :class="toast.ok ? 'toast-ok' : 'toast-err'" @click="dismissToast">
        {{ toast.message }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.main { width: 100%; max-width: 460px; display: flex; flex-direction: column; gap: 16px; }
header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }
.logo-sm { width: 40px; height: 40px; }
nav { display: flex; gap: 8px; align-items: center; }
.theme-toggle { width: 36px; height: 36px; display: flex; align-items: center; justify-content: center; padding: 4px; }
.theme-toggle svg { width: 20px; height: 20px; }
.link { background: none; border: none; color: var(--muted); cursor: pointer; font-size: 0.9rem; }
.link:hover { color: var(--brand); }
h2 { margin: 0 0 12px; font-size: 1rem; }
.device-drive { color: var(--muted); font-size: 0.85rem; text-align: center; }
.device-logo { width: 96px; height: 96px; object-fit: contain; display: block; margin: 0 auto 8px; }
.device-waiting {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 8px 0 4px;
}
.device-waiting-title { margin: 0; font-size: 1rem; color: var(--muted); }
.spinner {
  width: 20px; height: 20px;
  border: 2px solid var(--border);
  border-top-color: var(--brand);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
  flex-shrink: 0;
}
.spinner-lg {
  width: 40px; height: 40px;
  border-width: 3px;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.hint { color: var(--muted); margin: 0; font-size: 0.9rem; }
.pills { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.pill {
  display: flex; align-items: center; gap: 10px; padding: 12px;
  border: 2px solid var(--border, #e5e7eb); border-radius: 10px; cursor: pointer; font-size: 0.95rem;
  user-select: none;
}
.pill.active { border-color: var(--brand); background: var(--brand-tint); }
.pill img { width: 34px; height: 34px; }
.pill input {
  position: absolute; opacity: 0; width: 1px; height: 1px; margin: -1px;
  overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap;
}
.pill:focus-within { outline: 2px solid var(--brand); outline-offset: 2px; }
.count { color: var(--muted); font-size: 0.9rem; margin: 12px 0 0; }
.primary {
  width: 100%; padding: 14px; background: var(--brand); color: #fff;
  border: none; border-radius: 8px; font-size: 1.05rem; font-weight: 600; cursor: pointer;
}
.primary:disabled { opacity: 0.6; cursor: not-allowed; }
.update-banner { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.update-banner span { font-size: 0.95rem; }
.small { width: auto; padding: 8px 14px; font-size: 0.9rem; }
.toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 420px;
  width: calc(100% - 32px);
  padding: 12px 16px;
  border-radius: 10px;
  font-size: 0.9rem;
  text-align: center;
  line-height: 1.5;
  cursor: pointer;
  z-index: 100;
}
.toast-ok { color: var(--ok); background: var(--ok-tint); box-shadow: 0 4px 20px rgba(22, 163, 74, 0.15); }
.toast-err { color: var(--err); background: var(--err-tint); box-shadow: 0 4px 20px rgba(220, 38, 38, 0.15); }

.toast-enter-active { transition: all 0.3s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from { opacity: 0; transform: translateX(-50%) translateY(-16px); }
.toast-leave-to { opacity: 0; transform: translateX(-50%) translateY(-16px); }
</style>
