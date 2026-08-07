<script setup lang="ts">
import { computed } from 'vue'
import type { AppInfo, LogEntry } from '../types'

const props = defineProps<{
  visible: boolean
  appInfo: AppInfo | null
  logs: LogEntry[]
  loadingLogs: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'refresh-logs'): void
}>()

const formattedLogs = computed(() => {
  return props.logs
    .map((entry) => {
      const ts = new Date(entry.timestampUnixMs).toLocaleString('pt-BR')
      const level = entry.level.toUpperCase().padEnd(5, ' ')
      return `[${ts}] ${level} ${entry.message}`
    })
    .join('\n')
})

async function copyLogs() {
  if (!formattedLogs.value) return
  await navigator.clipboard.writeText(formattedLogs.value)
}

function onBackdropClick(event: MouseEvent) {
  if (event.target === event.currentTarget) {
    emit('close')
  }
}
</script>

<template>
  <Transition name="modal">
    <div v-if="visible" class="modal-backdrop" @click="onBackdropClick">
      <section class="modal-card" role="dialog" aria-modal="true" aria-label="Sobre o aplicativo">
        <header class="modal-header">
          <div class="title-wrap">
            <img src="/logo.svg" alt="MapaRadar" class="logo" />
            <div>
              <h2>Sobre</h2>
              <p>{{ appInfo?.name ?? 'Atualizador MapaRadar' }}</p>
            </div>
          </div>
          <button class="close-btn" @click="emit('close')" aria-label="Fechar">✕</button>
        </header>

        <section class="block">
          <h3>Informações do app</h3>
          <p><strong>Versão:</strong> {{ appInfo?.version ?? '—' }}</p>
          <p><strong>Plataforma:</strong> {{ appInfo ? `${appInfo.platform} (${appInfo.arch})` : '—' }}</p>
          <p><strong>Tauri:</strong> {{ appInfo?.tauriVersion ?? '—' }}</p>
        </section>

        <section class="block">
          <h3>Licenças</h3>
          <p><strong>Aplicativo:</strong> Proprietário (MapaRadar)</p>
          <p><strong>Base de radares:</strong> CC BY-NC-ND 4.0</p>
        </section>

        <section class="block logs">
          <div class="logs-header">
            <h3>Logs recentes (até 1000 linhas)</h3>
            <div class="logs-actions">
              <button class="action" @click="emit('refresh-logs')" :disabled="loadingLogs">Atualizar</button>
              <button class="action" @click="copyLogs" :disabled="!formattedLogs">Copiar</button>
            </div>
          </div>
          <pre>{{ loadingLogs ? 'Carregando logs...' : (formattedLogs || 'Sem logs ainda.') }}</pre>
        </section>
      </section>
    </div>
  </Transition>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(17, 24, 39, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  z-index: 200;
}

.modal-card {
  width: 100%;
  max-width: 520px;
  max-height: calc(100vh - 40px);
  overflow: auto;
  background: var(--card);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 12px 36px var(--shadow);
  padding: 20px;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
  margin-bottom: 16px;
}

.title-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo {
  width: 30px;
  height: 30px;
}

h2 {
  margin: 0;
  font-size: 1.1rem;
}

.title-wrap p {
  margin: 2px 0 0;
  color: var(--muted);
  font-size: 0.9rem;
}

.close-btn {
  background: none;
  border: none;
  color: var(--muted);
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
}

.block {
  border-top: 1px solid var(--border);
  padding-top: 12px;
  margin-top: 12px;
}

h3 {
  margin: 0 0 8px;
  font-size: 0.95rem;
}

p {
  margin: 0 0 6px;
  font-size: 0.9rem;
}

.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.logs-actions {
  display: flex;
  gap: 6px;
}

.action {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  border-radius: 8px;
  padding: 6px 10px;
  font-size: 0.8rem;
  cursor: pointer;
}

.action:disabled {
  opacity: 0.6;
  cursor: default;
}

pre {
  margin: 10px 0 0;
  border: 1px solid var(--border);
  background: var(--bg);
  border-radius: 8px;
  padding: 10px;
  max-height: 220px;
  overflow: auto;
  font-size: 0.76rem;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
