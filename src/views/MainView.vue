<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
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
const message = ref('')
const messageOk = ref(true)
let timer: number | undefined
let countRequest = 0

const currentDevice = computed(() => devices.value[0])
const radarTypes = computed(() => radarTypesString([...selected.value]))
const hasSelection = computed(() => radarTypes.value !== null)

function toggle(code: number) {
  message.value = ''
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

async function refreshDevices() {
  try {
    devices.value = await detectDevice()
  } catch {
    // transient detect failure: keep previous device state, don't flip UI
  }
}

async function doUpdate() {
  if (!currentDevice.value || radarTypes.value === null || busy.value) return
  busy.value = true
  message.value = ''
  try {
    const s: UpdateSummary = await updateDevice(currentDevice.value.kind, radarTypes.value)
    messageOk.value = true
    message.value = `Arquivo atualizado: ${s.filesWritten.join(', ')}`
  } catch (e) {
    messageOk.value = false
    message.value = toAppError(e).message
  } finally {
    busy.value = false
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
    message.value = 'Não foi possível carregar os tipos de alerta.'
    messageOk.value = false
  }
  selected.value = new Set(alertTypes.value.filter((t) => t.default).map((t) => t.code))
  refreshDevices()
  timer = window.setInterval(refreshDevices, 2000)
})

onUnmounted(() => window.clearInterval(timer))

watch(radarTypes, refreshCount)
</script>

<template>
  <div class="main">
    <header>
      <img src="/logo.svg" alt="MapaRadar" class="logo-sm" />
      <nav>
        <button class="link" @click="openUrl('https://maparadar.com/atualizador/#contato')">Ajuda</button>
        <button class="link" @click="openUrl('https://maparadar.com/atualizador/')">Sobre</button>
        <button class="link" @click="doLogout">Sair</button>
      </nav>
    </header>

    <section class="card device">
      <h2>Dispositivo</h2>
      <template v-if="currentDevice">
        <div class="device-name">{{ currentDevice.display }}</div>
        <div class="device-drive">{{ currentDevice.drive }}</div>
      </template>
      <p v-else class="hint">Conecte um dispositivo GPS compatível (iGO ou NDrive).</p>
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
        :disabled="!currentDevice || !hasSelection || busy"
        @click="doUpdate"
      >
        {{ busy ? 'Atualizando…' : 'Atualizar dispositivo' }}
      </button>
      <p v-if="message" :role="messageOk ? 'status' : 'alert'" class="message" :class="messageOk ? 'ok' : 'error'">
        {{ message }}
      </p>
    </section>
  </div>
</template>

<style scoped>
.main { width: 100%; max-width: 460px; display: flex; flex-direction: column; gap: 16px; }
header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }
.logo-sm { width: 40px; height: 40px; }
nav { display: flex; gap: 8px; }
.link { background: none; border: none; color: var(--muted); cursor: pointer; font-size: 0.9rem; }
.link:hover { color: var(--brand); }
h2 { margin: 0 0 12px; font-size: 1rem; }
.device-name { font-size: 1.2rem; font-weight: 700; }
.device-drive { color: var(--muted); font-size: 0.9rem; }
.hint { color: var(--muted); margin: 0; font-size: 0.9rem; }
.pills { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.pill {
  display: flex; align-items: center; gap: 8px; padding: 8px;
  border: 2px solid var(--border, #e5e7eb); border-radius: 8px; cursor: pointer; font-size: 0.85rem;
  user-select: none;
}
.pill.active { border-color: var(--brand); background: #fef2f2; }
.pill img { width: 28px; height: 28px; }
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
.message { text-align: center; font-size: 0.9rem; margin: 8px 0 0; }
.message.ok { color: var(--ok); }
.message.error { color: var(--err); }
</style>
