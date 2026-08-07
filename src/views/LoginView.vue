<script setup lang="ts">
import { ref } from 'vue'
import { openUrl } from '@tauri-apps/plugin-opener'
import AboutModal from '../components/AboutModal.vue'
import { getAppInfo, getLogs, login, toAppError } from '../api'
import type { AppInfo, LogEntry } from '../types'

const emit = defineEmits<{ (e: 'loggedIn'): void }>()

const username = ref('')
const password = ref('')
const error = ref('')
const busy = ref(false)
const showAbout = ref(false)
const appInfo = ref<AppInfo | null>(null)
const logs = ref<LogEntry[]>([])
const loadingLogs = ref(false)

async function submit() {
  if (busy.value) return
  error.value = ''
  busy.value = true
  try {
    await login(username.value, password.value)
    emit('loggedIn')
  } catch (e) {
    error.value = toAppError(e).message
  } finally {
    busy.value = false
  }
}

async function refreshLogs() {
  loadingLogs.value = true
  try {
    logs.value = await getLogs()
  } catch {
    logs.value = []
  } finally {
    loadingLogs.value = false
  }
}

async function openAbout() {
  try {
    appInfo.value = await getAppInfo()
  } catch {
    appInfo.value = null
  }
  await refreshLogs()
  showAbout.value = true
}

function openHelp() {
  openUrl('https://maparadar.com/atualizador.html')
}
</script>

<template>
  <div class="login-layout">
    <div class="card login-card">
      <img src="/logo.svg" alt="MapaRadar" class="logo" />
      <h1>Atualizador MapaRadar</h1>
      <p class="subtitle">Atualize os radares no seu GPS (iGO8 ou NDrive)</p>
      <form @submit.prevent="submit">
        <label>
          Usuário
          <input v-model="username" type="text" autocomplete="username" required />
        </label>
        <label>
          Senha
          <input v-model="password" type="password" autocomplete="current-password" required />
        </label>
        <p v-if="error" role="alert" class="error">{{ error }}</p>
        <button type="submit" class="submit-btn" :disabled="busy || !username || !password">
          {{ busy ? 'Entrando…' : 'Entrar' }}
        </button>
      </form>
    </div>

    <div class="meta-links">
      <button type="button" class="link" @click="openAbout">Sobre</button>
      <span aria-hidden="true">·</span>
      <button type="button" class="link" @click="openHelp">Ajuda</button>
    </div>

    <AboutModal
      :visible="showAbout"
      :app-info="appInfo"
      :logs="logs"
      :loading-logs="loadingLogs"
      @close="showAbout = false"
      @refresh-logs="refreshLogs"
    />
  </div>
</template>

<style scoped>
.login-layout {
  width: 100%;
  max-width: 460px;
}

.login-card {
  padding: 32px;
}

.logo { width: 136px; height: 136px; display: block; margin: 0 auto 10px; }
h1 { text-align: center; font-size: 1.45rem; margin: 0 0 8px; }
.subtitle { text-align: center; color: var(--muted); margin: 0 auto 24px; font-size: 0.92rem; line-height: 1.4; max-width: 300px; }
form { display: flex; flex-direction: column; gap: 16px; }
label { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; color: var(--muted); }
input { padding: 11px; border: 1px solid var(--border); border-radius: 8px; font-size: 1rem; color: var(--text); background: var(--card); transition: background-color 0.2s ease, color 0.2s ease, border-color 0.2s ease; }
.submit-btn { margin-top: 6px; padding: 12px; background: var(--brand); color: #fff; border: none; border-radius: 8px; font-size: 1rem; font-weight: 600; cursor: pointer; }
.submit-btn:disabled { opacity: 0.6; cursor: not-allowed; }
.error { color: var(--err); font-size: 0.85rem; margin: 0; }
.meta-links {
  margin-top: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--muted);
  font-size: 0.85rem;
}
.link {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 0.85rem;
  padding: 2px;
}
.link:hover {
  color: var(--brand);
}
</style>
