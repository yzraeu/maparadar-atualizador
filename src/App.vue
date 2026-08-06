<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { sessionStatus } from './api'
import LoginView from './views/LoginView.vue'
import MainView from './views/MainView.vue'

const loggedIn = ref<boolean | null>(null)

onMounted(async () => {
  const s = await sessionStatus()
  loggedIn.value = s !== null
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
    <div v-else class="loading">Carregando…</div>
  </div>
</template>

<style scoped>
.loading {
  color: var(--muted);
}
</style>
