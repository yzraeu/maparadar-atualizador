import { ref } from 'vue'

export const theme = ref<'light' | 'dark'>('light')

function applyTheme(t: 'light' | 'dark') {
  theme.value = t
  document.documentElement.setAttribute('data-theme', t)
  localStorage.setItem('maparadar-theme', t)
}

export function initTheme() {
  const saved = localStorage.getItem('maparadar-theme')
  if (saved === 'light' || saved === 'dark') {
    applyTheme(saved)
  } else if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
    applyTheme('dark')
  } else {
    applyTheme('light')
  }
}

export function toggleTheme() {
  applyTheme(theme.value === 'light' ? 'dark' : 'light')
}
