import { createApp } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { initTheme } from './theme'
import App from './App.vue'
import './styles.css'

if (import.meta.env.PROD) {
  document.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement
    if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
      e.preventDefault()
    }
  })
}

document.addEventListener('dragstart', (e) => {
  if ((e.target as HTMLElement).tagName === 'IMG') {
    e.preventDefault()
  }
})

getCurrentWindow().setIcon('/icons/32x32.png')

initTheme()

createApp(App).mount('#app')
