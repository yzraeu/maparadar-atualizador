<script setup lang="ts">
import { ref } from 'vue'
import { login, toAppError } from '../api'

const emit = defineEmits<{ (e: 'loggedIn'): void }>()

const username = ref('')
const password = ref('')
const error = ref('')
const busy = ref(false)

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
</script>

<template>
  <div class="card">
    <img src="/logo.svg" alt="MapaRadar" class="logo" />
    <h1>MapaRadar Atualizador</h1>
    <p class="subtitle">Atualize os radares no seu GPS (iGO ou NDrive)</p>
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
      <button type="submit" :disabled="busy || !username || !password">
        {{ busy ? 'Entrando…' : 'Entrar' }}
      </button>
    </form>
  </div>
</template>

<style scoped>
.logo { width: 96px; height: 96px; display: block; margin: 0 auto 8px; }
h1 { text-align: center; font-size: 1.35rem; margin: 0 0 4px; }
.subtitle { text-align: center; color: var(--muted); margin: 0 0 20px; font-size: 0.9rem; }
form { display: flex; flex-direction: column; gap: 14px; }
label { display: flex; flex-direction: column; gap: 6px; font-size: 0.85rem; color: var(--muted); }
input { padding: 10px; border: 1px solid var(--border); border-radius: 8px; font-size: 1rem; color: var(--text); background: var(--card); transition: background-color 0.2s ease, color 0.2s ease, border-color 0.2s ease; }
button { padding: 12px; background: var(--brand); color: #fff; border: none; border-radius: 8px; font-size: 1rem; font-weight: 600; cursor: pointer; }
button:disabled { opacity: 0.6; cursor: not-allowed; }
.error { color: var(--err); font-size: 0.85rem; margin: 0; }
</style>
