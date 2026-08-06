<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { sessionStatus } from './api'
import LoginView from './views/LoginView.vue'
import MainView from './views/MainView.vue'

const loggedIn = ref<boolean | null>(null)

onMounted(async () => {
  try {
    const s = await sessionStatus()
    loggedIn.value = s !== null
  } catch {
    loggedIn.value = false
  }
})

function onLoggedIn() {
  loggedIn.value = true
}

function onLogout() {
  loggedIn.value = false
}
</script>

<template>
  <div class="app">
    <LoginView v-if="loggedIn === false" @logged-in="onLoggedIn" />
    <MainView v-else-if="loggedIn === true" @logout="onLogout" />
    <div v-else role="status" class="loading">
      <span class="spinner"></span>
      Carregando…
    </div>
  </div>
</template>

<style scoped>
.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  color: var(--muted);
  font-size: 1rem;
}
.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border);
  border-top-color: var(--brand);
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
